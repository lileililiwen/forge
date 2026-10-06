//! forge — session-gated, JSON-only portfolio controls and truthful
//! cross-project evidence views (`forge-web-portfolio-controls/0.1.0`).
//!
//! This module projects Forge-owned portfolio metadata (tags, relations,
//! reviews, goals, share state) and cross-project source evidence (catalog,
//! gap report, fleet/inventory sources, governance selection, analytics
//! support, provider matrix, readiness profiles, interest activation) to the
//! authenticated web operator. It reuses the crate's own typed in-process
//! Core functions only.
//!
//! Security boundary (see `.ai-rules/concerns/security.md`):
//! - Nothing here runs a shell, `sh -c`, an interpreter, or the `forge`
//!   executable, and no route accepts arbitrary command text or a path.
//! - A project is addressed only by an opaque, validated `id`; the root is
//!   resolved server-side and never serialized to the browser.
//! - Provider/readiness evidence is **non-live by default**: landing on the
//!   evidence view performs no probe and never runs a native build; each is
//!   labelled `not_run` until an operator runs it in a terminal.
//! - Imported, source-owned evidence is read-only. There is no route that
//!   edits a source snapshot; the one write surface for `evidence` refuses
//!   the mutation and leaves the snapshot unchanged.
//! - Absolute filesystem paths are never serialized: every projected string
//!   is scrubbed of local-path tokens before it leaves this module, and
//!   Core `Display` text is never echoed on failure.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::{ApiRequest, ApiResponse};
use crate::catalog::{self, CatalogSourceSelection, SourceKind};
use crate::core::{validate_project_id, ForgeError};
use crate::doctor::gaps;
use crate::registry::Registry;

/// Versioned portfolio-controls contract. Additive only.
pub const CONTRACT_VERSION: &str = "forge-web-portfolio-controls/0.1.0";

/// The fixed set of write actions a browser may post to
/// `/v1/admin/portfolio/{id}/{action}`. Any other segment is refused — the
/// action is a validated key, never a path or shell token.
const WRITE_ACTIONS: &[&str] = &["tags", "relations", "reviews", "goals", "evidence"];
/// The fixed set of read sub-resources on `/v1/admin/portfolio/{id}/{kind}`.
const READ_KINDS: &[&str] = &["evidence"];

/// Env var naming the aggregate activation cohort threshold for the interest
/// evidence section; falls back to [`DEFAULT_INTEREST_THRESHOLD`].
const INTEREST_THRESHOLD_ENV: &str = "FORGE_INTEREST_ACTIVATION_THRESHOLD";
const DEFAULT_INTEREST_THRESHOLD: u64 = 100;

// --- read: portfolio fleet -----------------------------------------------

/// `GET /v1/admin/portfolio` — every registered project's user-owned record,
/// its tags and the displayed state of every evidence source. Optional
/// `tag`/`lifecycle`/`confidence` query filters narrow the list; an invalid
/// filter is a typed `400`, never a silently widened result set. Source-owned
/// evidence appears read-only with its effective (possibly stale/unavailable)
/// status and observation time; it is never editable from here.
pub fn list(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    let filter = match parse_portfolio_filter(request.query.as_deref()) {
        Ok(filter) => filter,
        Err(reason) => return refuse(400, "portfolio-invalid", &reason),
    };
    match registry.portfolio_fleet(&filter, Utc::now()) {
        Ok(rows) => {
            let items: Vec<Value> = rows
                .iter()
                .map(|row| {
                    json!({
                        "project_id": row.profile.project_id,
                        "lifecycle": row.profile.lifecycle.map(|v| v.label()),
                        "confidence": row.profile.confidence.map(|v| v.label()),
                        "next_action": row.profile.next_action,
                        "blocker": row.profile.blocker,
                        "reviewed_at": row.profile.reviewed_at,
                        "tags": row.tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
                        "evidence": row.evidence.iter().map(|e| json!({
                            "source_system": e.source_system,
                            "source_revision": e.source_revision,
                            "status": e.status.label(),
                            "observed_at": e.observed_at,
                        })).collect::<Vec<_>>(),
                        "evidence_summary": row.evidence_summary(),
                    })
                })
                .collect();
            let with_evidence = items
                .iter()
                .filter(|item| {
                    item["evidence"]
                        .as_array()
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false)
                })
                .count();
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "generated_at": Utc::now().to_rfc3339(),
                "filter": filter_echo(&filter),
                "summary": { "projects": items.len(), "with_evidence": with_evidence },
                "projects": items,
            }))
        }
        Err(err) => typed_refusal(&err),
    }
}

// --- read: single-project portfolio view ---------------------------------

/// `GET /v1/admin/portfolio/{id}` — the full portfolio projection for one
/// managed project: user-owned fields, tags, goal membership, relations in
/// both directions, the newest snapshot per source (read-only, effective
/// status) and review history. Source-owned evidence is shown, never edited.
pub fn detail(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    match registry.portfolio_project_view(id, Utc::now()) {
        Ok(view) => {
            let value = serde_json::to_value(&view).unwrap_or(Value::Null);
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "management": management_label(id),
                "portfolio": value,
            }))
        }
        Err(err) => typed_refusal(&err),
    }
}

/// `GET /v1/admin/portfolio/{id}/{kind}` — a read-only sub-resource view.
/// Today only `evidence` (the source-owned snapshots, with provenance and
/// freshness) is served; every other kind is a `404`.
pub fn read_item(db_path: &Path, id: &str, kind: &str) -> ApiResponse {
    if !READ_KINDS.contains(&kind) {
        return refuse(
            404,
            "portfolio-route-not-found",
            "no portfolio resource matches this path",
        );
    }
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }
    let now = Utc::now();
    match registry.portfolio_current_snapshots(id) {
        Ok(snapshots) => {
            let items: Vec<Value> = snapshots
                .iter()
                .map(|snapshot| {
                    let status = crate::portfolio::effective_status(
                        snapshot.status,
                        snapshot.stale_after.as_deref(),
                        now,
                    );
                    json!({
                        "source_system": snapshot.source_system,
                        "source_revision": snapshot.source_revision,
                        "observed_at": snapshot.observed_at,
                        "status": status.label(),
                        "editable": false,
                        "note": "imported evidence is source-owned and append-only; it cannot be edited from the browser",
                    })
                })
                .collect();
            scrub(json!({
                "contract": CONTRACT_VERSION,
                "project_id": id,
                "evidence": items,
            }))
        }
        Err(err) => typed_refusal(&err),
    }
}

// --- write: Forge-owned metadata -----------------------------------------

/// `POST /v1/admin/portfolio/{id}/{action}` — a Forge-owned metadata
/// mutation (`tags`, `relations`, `reviews`, `goals`). Each maps to one
/// typed Core write; nothing is executed, and no source-owned snapshot is
/// touched. The `evidence` action is the honest refusal path: imported
/// observations are append-only and cannot be edited from the browser, so
/// the snapshot is preserved unchanged.
pub fn write_item(db_path: &Path, id: &str, action: &str, body: &Value) -> ApiResponse {
    if !WRITE_ACTIONS.contains(&action) {
        return refuse(
            404,
            "portfolio-route-not-found",
            "no portfolio action matches this path",
        );
    }
    if action == "evidence" {
        // Source-owned evidence is never editable through the browser: refuse
        // before opening a write, so the snapshot provably stays unchanged.
        return refuse(
            403,
            "portfolio-source-owned",
            "imported evidence is source-owned and append-only; the browser cannot edit it and the stored snapshot was left unchanged.",
        );
    }

    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    if let Err(response) = require_id(&registry, id) {
        return response;
    }

    let outcome = match action {
        "tags" => {
            let name = match required_str(body, "name") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let color = optional_str(body, "color").map(|value| value.to_string());
            registry
                .portfolio_add_tag(id, &name, color.as_deref())
                .map(|tag| json!({ "tag": tag }))
        }
        "relations" => {
            let to = match required_str(body, "to") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let raw_type = match required_str(body, "type") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let relation_type = match crate::portfolio::RelationType::parse(&raw_type) {
                Ok(value) => value,
                Err(reason) => return refuse(400, "portfolio-invalid", &reason),
            };
            let note = optional_str(body, "note").map(|value| value.to_string());
            registry
                .portfolio_add_relation(id, &to, relation_type, note.as_deref())
                .map(|relation| json!({ "relation": relation }))
        }
        "reviews" => {
            let raw_confidence = match required_str(body, "confidence") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let confidence = match crate::portfolio::Confidence::parse(&raw_confidence) {
                Ok(value) => value,
                Err(reason) => return refuse(400, "portfolio-invalid", &reason),
            };
            let lifecycle = match optional_str(body, "lifecycle") {
                Some(raw) => match crate::portfolio::Lifecycle::parse(raw) {
                    Ok(value) => Some(value),
                    Err(reason) => return refuse(400, "portfolio-invalid", &reason),
                },
                None => None,
            };
            let note = optional_str(body, "note").map(|value| value.to_string());
            let next_action = optional_str(body, "next_action").map(|value| value.to_string());
            let blocker = optional_str(body, "blocker").map(|value| value.to_string());
            let write = crate::registry::PortfolioWrite {
                lifecycle,
                confidence: Some(confidence),
                next_action,
                blocker,
            };
            registry
                .portfolio_write(id, &write)
                .and_then(|profile| {
                    registry
                        .portfolio_record_review(id, confidence, note.as_deref())
                        .map(|review| (profile, review))
                })
                .map(|(profile, review)| json!({ "profile": profile, "review": review }))
        }
        "goals" => {
            let title = match required_str(body, "title") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            let raw_status = match required_str(body, "status") {
                Ok(value) => value.to_string(),
                Err(response) => return response,
            };
            // Validate the status before touching the registry so a bad
            // status changes nothing (linking a goal would otherwise create
            // it with a forced status first).
            if let Err(reason) = crate::portfolio::validate_goal_status(&raw_status) {
                return refuse(400, "portfolio-invalid", &reason);
            }
            let description = optional_str(body, "description").map(|value| value.to_string());
            registry
                .portfolio_link_goal(&title, id)
                .and_then(|linked| {
                    // Re-stamp the operator's requested status/description
                    // onto the goal, then read it back with its membership.
                    registry.portfolio_add_goal(&title, &raw_status, description.as_deref())?;
                    registry.portfolio_goal(linked.goal_id)
                })
                .map(|goal| json!({ "goal": goal }))
        }
        other => unreachable!("action {other} is not in WRITE_ACTIONS"),
    };

    match outcome {
        Ok(data) => scrub(json!({
            "contract": CONTRACT_VERSION,
            "project_id": id,
            "action": action,
            "effect": "forge-owned-write",
            "actor": "global-admin",
            "recorded_at": Utc::now().to_rfc3339(),
            "result": data,
        })),
        Err(err) => typed_refusal(&err),
    }
}

// --- read: cross-project evidence bundle ---------------------------------

/// `GET /v1/admin/portfolio/evidence` — one honest cross-project evidence
/// surface assembled entirely from in-process Core reads. Every section
/// carries its own source, provenance and actual status; an unavailable
/// source contributes a status rather than hiding the others, and no probe,
/// native build or external adapter runs on page load.
pub fn evidence(db_path: &Path) -> ApiResponse {
    let registry = match open(db_path) {
        Ok(registry) => registry,
        Err(response) => return response,
    };
    let managed_ids: Vec<String> = registry
        .list()
        .map(|records| records.into_iter().map(|record| record.id).collect())
        .unwrap_or_default();

    let now = Utc::now();

    // Catalog + gap report over the local source only. A local selection
    // never reads a git working tree, so no external absolute path exists to
    // leak; findings still synthesize an `unavailable` row for any unreadable
    // source rather than dropping it.
    let catalog_section = build_catalog_section(db_path, now);
    let gaps_section = build_gaps_section(db_path, now);

    // Fleet + portable-inventory + workspace source statuses, reusing the
    // already-path-free fleet envelope.
    let fleet_section = match super::fleet::load(db_path) {
        Ok(envelope) => json!({
            "state": "available",
            "sources": envelope.get("sources").cloned().unwrap_or_else(|| json!([])),
            "summary": envelope.get("summary").cloned().unwrap_or(Value::Null),
        }),
        Err(_) => json!({
            "state": "unavailable",
            "note": "the fleet and inventory sources could not be read; nothing was changed.",
        }),
    };

    // Governance provider selection/status, derived from the process
    // environment rather than any project path. No adapter is invoked.
    let workspace_configured = std::env::var(crate::governance::WORKSPACE_ROOT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some();
    let governance_section = json!({
        "selected": if workspace_configured { crate::governance::WORKSPACE_GOVERNANCE_PROVIDER_ID } else { crate::governance::LOCAL_PROVIDER_ID },
        "providers": [
            { "provider": crate::governance::LOCAL_PROVIDER_ID, "status": "enabled", "enabled": true, "protocol_version": crate::governance::GOVERNANCE_CONTRACT_VERSION },
            { "provider": crate::governance::WORKSPACE_GOVERNANCE_PROVIDER_ID, "status": if workspace_configured { "enabled" } else { "unconfigured" }, "enabled": workspace_configured, "protocol_version": crate::governance::GOVERNANCE_CONTRACT_VERSION },
        ],
        "note": "governance selection is a local configuration observation; the external adapter is not contacted on page load.",
    });

    // Analytics provider support matrix. Support status is static catalog
    // data; no external analytics adapter is contacted here.
    let analytics_section = json!({
        "live": false,
        "note": "provider support is read from the analytics catalog; live analytics metrics require an explicit adapter call in a terminal.",
        "providers": analytics_provider_rows(),
    });

    // Provider evidence matrix, explicitly non-live: every row reports
    // `not_run` (or its honest disabled/unavailable state) and no probe is
    // performed by building this response.
    let provider_section = serde_json::to_value(crate::provider::matrix(false))
        .unwrap_or_else(|_| json!({ "error": "provider matrix could not be serialized" }));

    // Readiness profiles: only the supported profile ids are listed and the
    // state is `not_run`. Running the matrix executes native build checks,
    // which is never done on page load.
    let readiness_section = json!({
        "state": "not_run",
        "profiles": crate::readiness::matrix_profile_ids(),
        "note": "running the readiness matrix executes native build checks in a terminal; it is never run by loading this page.",
    });

    // Interest activation readiness over the managed fleet, honoring the
    // configured aggregate cohort threshold. Below-threshold cohorts are
    // withheld by Core and reported with the honest reason rather than a
    // fabricated pass.
    let threshold = std::env::var(INTEREST_THRESHOLD_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTEREST_THRESHOLD);
    let interest_section = if managed_ids.is_empty() {
        json!({
            "state": "no_projects",
            "threshold": threshold,
            "note": "no managed projects are registered, so there is no aggregate interest evidence to evaluate.",
        })
    } else {
        let metric = crate::portfolio::interest::InterestMetric::UniqueVisitors;
        match crate::portfolio::interest_report::activation_readiness(
            &registry,
            &managed_ids,
            metric,
            Some(threshold),
            None,
            None,
            crate::portfolio::interest::DEFAULT_STALE_AFTER_DAYS,
            now,
        ) {
            Ok(report) => json!({
                "state": "evaluated",
                "metric": report.metric,
                "threshold": report.threshold,
                "ready": report.verdicts.iter().filter(|v| v.readiness.is_ready()).count(),
                "not_ready": report.verdicts.iter().filter(|v| !v.readiness.is_ready()).count(),
                "verdicts": report.verdicts.iter().map(|verdict| json!({
                    "project_id": verdict.project_id,
                    "readiness": verdict.readiness.label(),
                    "withheld": !verdict.readiness.is_ready(),
                    "reasons": verdict.reasons.iter().map(|r| r.reason.label()).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
            Err(err) => json!({
                "state": "unavailable",
                "threshold": threshold,
                "note": status_reason(&err),
            }),
        }
    };

    scrub(json!({
        "contract": CONTRACT_VERSION,
        "generated_at": now.to_rfc3339(),
        "live_probes": false,
        "sections": {
            "catalog": catalog_section,
            "gaps": gaps_section,
            "fleet": fleet_section,
            "governance": governance_section,
            "analytics": analytics_section,
            "provider": provider_section,
            "readiness": readiness_section,
            "interest": interest_section,
        },
    }))
}

fn build_catalog_section(db_path: &Path, now: DateTime<Utc>) -> Value {
    let selection = CatalogSourceSelection {
        kinds: vec![SourceKind::Local],
        git_repositories: Vec::new(),
        workspace_registry: None,
        inventory: None,
        github_repositories: Vec::new(),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: crate::fleet::DEFAULT_MAX_AGE_SECONDS,
        now,
    });
    let sources: Vec<Value> = bundle
        .statuses
        .iter()
        .map(|status| {
            json!({
                "source": status.source,
                "state": status.state,
                "records": status.records,
            })
        })
        .collect();
    json!({
        "contract": catalog::CATALOG_CONTRACT_VERSION,
        "observed_at": bundle.observed_at,
        "record_count": bundle.records.len(),
        "tag_counts": catalog::tag_counts(&bundle.records),
        "language_counts": catalog::language_counts(&bundle.records),
        "sources": sources,
    })
}

fn build_gaps_section(db_path: &Path, now: DateTime<Utc>) -> Value {
    let selection = CatalogSourceSelection {
        kinds: vec![SourceKind::Local],
        git_repositories: Vec::new(),
        workspace_registry: None,
        inventory: None,
        github_repositories: Vec::new(),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: crate::fleet::DEFAULT_MAX_AGE_SECONDS,
        now,
    });
    let mut records = bundle.records.clone();
    records.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });
    let report = gaps::build_report(&records, &bundle.statuses, None);
    let mut by_status: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for finding in &report.findings {
        *by_status
            .entry(finding.status.id().to_string())
            .or_default() += 1;
    }
    json!({
        "contract": report.contract,
        "total": report.findings.len(),
        "clean": report.clean(),
        "by_status": by_status,
        "findings": report.findings.iter().take(200).map(|finding| json!({
            "id": finding.id,
            "project_id": finding.project_id,
            "category": finding.category.id(),
            "status": finding.status.id(),
            "subject": finding.subject,
            "remediation_class": finding.remediation_class.id(),
            "source": finding.source,
            "observed_at": finding.observed_at,
            "freshness": finding.freshness.id(),
            "detail": finding.detail,
        })).collect::<Vec<_>>(),
    })
}

fn analytics_provider_rows() -> Vec<Value> {
    use crate::analytics::{
        provider_label, provider_support_status, AnalyticsProvider, ProviderSupportStatus,
    };
    let providers = [
        AnalyticsProvider::UnifiedContent,
        AnalyticsProvider::GithubAnalytics,
        AnalyticsProvider::Notion,
        AnalyticsProvider::Confluence,
        AnalyticsProvider::GitlabAnalytics,
        AnalyticsProvider::CodebergAnalytics,
    ];
    providers
        .into_iter()
        .map(|provider| {
            let supported = matches!(
                provider_support_status(provider),
                ProviderSupportStatus::Supported
            );
            json!({
                "provider": provider_label(provider),
                "support": if supported { "supported" } else { "planned" },
                "live": false,
            })
        })
        .collect()
}

// --- shared helpers -------------------------------------------------------

fn open(db_path: &Path) -> Result<Registry, ApiResponse> {
    Registry::open(db_path).map_err(|_| unavailable())
}

/// Validate the opaque id shape and confirm the project is registered. The
/// browser never sends a path; a bad id is `400`, an unmanaged/observed-only
/// id is an honest `404`. The offending id is never echoed back.
fn require_id(registry: &Registry, id: &str) -> Result<(), ApiResponse> {
    if validate_project_id(id).is_err() {
        return Err(refuse(
            400,
            "portfolio-invalid",
            "the project id is not a valid identifier; it may not contain a path.",
        ));
    }
    if registry.inspect(id).is_ok() {
        return Ok(());
    }
    Err(refuse(
        404,
        "portfolio-unmanaged-project",
        "this project is not managed by this Forge registry; it is either unknown or observed-only, so it cannot be shown or operated in the browser.",
    ))
}

fn parse_portfolio_filter(
    query: Option<&str>,
) -> Result<crate::portfolio::PortfolioFilter, String> {
    use crate::portfolio::{Confidence, Lifecycle, PortfolioFilter};
    let mut filter = PortfolioFilter::default();
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|p| !p.is_empty())
    {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = super::percent_decode(value);
        match key.trim() {
            "tag" => filter.tag = Some(value.trim().to_string()),
            "lifecycle" => filter.lifecycle = Some(Lifecycle::parse(value.trim()).map_err(|r| r)?),
            "confidence" => {
                filter.confidence = Some(Confidence::parse(value.trim()).map_err(|r| r)?)
            }
            other => {
                return Err(format!(
                    "unknown portfolio filter `{other}`; expected tag, lifecycle or confidence"
                ))
            }
        }
    }
    Ok(filter)
}

fn filter_echo(filter: &crate::portfolio::PortfolioFilter) -> Value {
    json!({
        "tag": filter.tag,
        "lifecycle": filter.lifecycle.map(|v| v.label()),
        "confidence": filter.confidence.map(|v| v.label()),
    })
}

fn management_label(id: &str) -> &'static str {
    if id == "forge" {
        "self"
    } else {
        "managed"
    }
}

fn required_str<'a>(body: &'a Value, field: &str) -> Result<&'a str, ApiResponse> {
    body.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            refuse(
                400,
                "portfolio-invalid",
                &format!("requires a `{field}` field"),
            )
        })
}

fn optional_str<'a>(body: &'a Value, field: &str) -> Option<&'a str> {
    body.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

/// Map a typed Core error to a safe, path-free portfolio refusal. The Core
/// `Display` text can legitimately name an absolute path, so it is only ever
/// included after the local-path scrub, and only for validation-style
/// refusals that carry no path in the first place.
fn typed_refusal(err: &ForgeError) -> ApiResponse {
    let code = err.code();
    let (status, message): (u16, String) = match code {
        "unknown-project" => (
            404,
            "the selected project is not managed by this Forge registry; nothing was changed."
                .to_string(),
        ),
        "path-unavailable" => (
            404,
            "the registered project directory is not currently available on this host; nothing was changed."
                .to_string(),
        ),
        "portfolio-invalid" | "portfolio-share-invalid" | "portfolio-interest-invalid" => {
            (400, status_reason(err))
        }
        "portfolio-share-conflict" | "portfolio-interest-conflict" | "portfolio-activation-not-ready" => {
            (409, status_reason(err))
        }
        _ => (
            503,
            "the portfolio service could not complete this operation; nothing was changed."
                .to_string(),
        ),
    };
    let message = redact_local_paths(&message);
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code, "message": message },
            "effect": "none",
        }),
    )
}

/// A short, path-free human reason for an error. Core validation reasons name
/// fields, not paths; anything that still looks like a local path is redacted
/// by [`redact_local_paths`] before display.
fn status_reason(err: &ForgeError) -> String {
    let text = err.to_string();
    // Take the leading clause only, so a trailing path segment is dropped even
    // before the scrubber runs.
    text.split(';')
        .next()
        .unwrap_or("the operation could not be completed")
        .to_string()
}

fn refuse(status: u16, code: &str, reason: &str) -> ApiResponse {
    let reason = redact_local_paths(reason);
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code, "message": reason },
            "effect": "none",
        }),
    )
}

fn unavailable() -> ApiResponse {
    refuse(
        503,
        "portfolio-unavailable",
        "the portfolio service could not open the registry; no rows or journal entries were changed.",
    )
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. Applied over the
/// whole serialized projection as a second layer beneath the path-free Core
/// reads, so no filesystem location can ever leave this module.
fn redact_local_paths(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let is_abs = token.starts_with('/')
                || (token.len() > 2
                    && token.as_bytes()[1] == b':'
                    && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                || token.contains("=/")
                || token.contains(":\\");
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Recursively scrub every string in the projection of absolute-path tokens.
/// This is the belt-and-suspenders guarantee for the no-path-leakage rule:
/// even if a source unexpectedly surfaces a path, it never reaches the wire.
fn scrub(value: Value) -> ApiResponse {
    ApiResponse::json(200, scrub_value(value))
}

fn scrub_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_local_paths(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(scrub_value).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key, scrub_value(item));
            }
            Value::Object(out)
        }
        other => other,
    }
}
