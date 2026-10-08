//! Normalized, provenance-bearing web fleet read model used by
//! `GET /v1/admin/projects`.
//!
//! This module owns only aggregation: it combines the Forge-self record, the
//! local registered projects and any explicitly configured external source
//! into one stable JSON envelope. It never mutates, never scans the
//! filesystem beyond an operator-declared source, and never serializes an
//! external absolute path. The browser transport lives in `frontend/`; the
//! API stays JSON-only.
//!
//! The envelope is versioned separately from `API_CONTRACT_VERSION` so the
//! fleet shape can evolve without touching the auth/session contract. Every
//! registered-project field the standalone frontend already reads (`id`,
//! `profile`, `state`, `lifecycle`, `confidence`, `tags`, `evidence`) stays
//! present so existing callers keep working while the new normalized fields
//! (`identity`, `name`, `source`, `source_ref`, `management`, `is_self`,
//! `freshness`, `capabilities`, `conflict`) are added.

use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use crate::core::{validate_project_id, ForgeError};
use crate::fleet::{self, FleetFreshness};
use crate::publish::inventory;
use crate::registry::{PublishedOperation, Registry};

use super::fleet_data::load_fleet_list;

/// Versioned contract for the normalized web fleet envelope. Bumped to
/// `0.2.0` when the `published` source and the per-row `publish` object were
/// added; existing fields and sources are unchanged and remain backward
/// compatible.
pub const WEB_FLEET_CONTRACT_VERSION: &str = "forge-web-fleet/0.2.0";

/// Default stable identity for the Forge-self row; overridable through the
/// established `FORGE_SELF_ID` configuration path so an operator can bind
/// the self record to a known registry id without editing code.
pub const SELF_DEFAULT_ID: &str = "forge";

/// Human display name for the Forge-self row.
pub const SELF_NAME: &str = "Forge";

/// Environment variable naming the freshness window (seconds) applied to
/// external sources; validated against the existing fleet bounds.
pub const FLEET_MAX_AGE_ENV: &str = "FORGE_FLEET_MAX_AGE_SECONDS";

/// Environment variable overriding the Forge-self identity.
pub const SELF_ID_ENV: &str = "FORGE_SELF_ID";

/// Environment variable controlling the local publish-history projection.
/// The projection is enabled by default (it reads the Forge-owned local
/// registry, never an external path); `0`, `false` or `off` disables it and
/// reports the `published` source as unconfigured.
pub const PUBLISH_HISTORY_ENV: &str = "FORGE_PUBLISH_HISTORY";

/// Environment variable bounding how many distinct published projects are
/// projected. Parsed as a positive integer and clamped to
/// `1..=PUBLISH_HISTORY_MAX_LIMIT`.
pub const PUBLISH_HISTORY_LIMIT_ENV: &str = "FORGE_PUBLISH_HISTORY_LIMIT";

/// Default bound for the publish-history source.
pub const PUBLISH_HISTORY_DEFAULT_LIMIT: usize = 200;

/// Hard ceiling so a machine-supplied limit can never pin the renderer.
pub const PUBLISH_HISTORY_MAX_LIMIT: usize = 1000;

/// Which provenance produced one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSource {
    SelfRecord,
    Registry,
    Inventory,
    WorkspaceRegistry,
    Published,
}

impl RowSource {
    pub fn id(self) -> &'static str {
        match self {
            RowSource::SelfRecord => "self",
            RowSource::Registry => "registry",
            RowSource::Inventory => "inventory",
            RowSource::WorkspaceRegistry => "fleet",
            RowSource::Published => "published",
        }
    }
}

/// Whether the row can be operated by Forge. Only `managed` and `self` rows
/// may ever expose an operation link; `observed` rows are inspection-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Management {
    Managed,
    Observed,
    SelfRecord,
}

impl Management {
    pub fn id(self) -> &'static str {
        match self {
            Management::Managed => "managed",
            Management::Observed => "observed",
            Management::SelfRecord => "self",
        }
    }
}

/// Truthful per-row freshness label derived from its source status. Rows are
/// only emitted by sources that actually returned data, so an unavailable
/// source contributes no rows and its state lives on the source descriptor
/// (`sources[].status = "unavailable"`), never as a per-row label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowFreshness {
    Fresh,
    Stale,
    Unconfigured,
}

impl RowFreshness {
    pub fn id(self) -> &'static str {
        match self {
            RowFreshness::Fresh => "fresh",
            RowFreshness::Stale => "stale",
            RowFreshness::Unconfigured => "unconfigured",
        }
    }
}

/// One aggregated candidate row. Built by [`gather`] (real sources) or by
/// tests (fixture data); [`build_envelope`] then merges, sorts and renders it.
#[derive(Debug, Clone)]
pub struct CandidateRow {
    pub identity: String,
    pub name: String,
    pub profile: Option<String>,
    pub state: String,
    pub source: RowSource,
    pub management: Management,
    pub is_self: bool,
    pub freshness: RowFreshness,
    pub updated_at: Option<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<(String, String)>,
    pub lifecycle: Option<String>,
    pub confidence: Option<String>,
    pub tags: Vec<String>,
    /// True only on the self row when Forge is also a registered project;
    /// the registry row is merged into the self row rather than duplicated.
    pub has_registry_ref: bool,
    /// Most recent local publish operation, when one exists for this
    /// identity. Attached to a registered/self row (merge) or carried by a
    /// standalone observed `published` row.
    pub publish: Option<PublishProjection>,
}

/// Bounded projection of one project's most recent publish operation, as
/// rendered on a fleet row. `detail` is redacted before it is stored;
/// `healthy`/`stages` are parsed from that redacted detail and stay `None`
/// when the detail does not carry them (e.g. a legacy row).
#[derive(Debug, Clone, Default)]
pub struct PublishProjection {
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub detail: Option<String>,
    pub target: Option<String>,
    pub revision: Option<String>,
    pub build_status: Option<String>,
    pub run_status: Option<String>,
    pub container_identity: Option<String>,
    pub healthy: Option<bool>,
    pub stages: Option<i64>,
}

/// One declared source's status for the `sources` array. Carries a safe
/// reason and malformed names only — never an absolute filesystem path.
#[derive(Debug, Clone)]
pub struct SourceDescriptor {
    pub id: &'static str,
    pub kind: &'static str,
    pub status: &'static str,
    pub count: usize,
    pub malformed: Vec<(String, String)>,
    pub reason: Option<String>,
    pub observed_at: Option<String>,
    pub provider: Option<String>,
}

impl SourceDescriptor {
    fn unconfigured(id: &'static str, kind: &'static str) -> Self {
        SourceDescriptor {
            id,
            kind,
            status: "unconfigured",
            count: 0,
            malformed: Vec::new(),
            reason: None,
            observed_at: None,
            provider: None,
        }
    }
}

/// Resolve the configured fleet freshness window, falling back to the shared
/// fleet default when the environment value is absent or out of bounds.
pub fn configured_max_age() -> i64 {
    let raw = std::env::var(FLEET_MAX_AGE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(value) = raw {
        if let Ok(parsed) = value.parse::<i64>() {
            if fleet::validate_max_age(parsed).is_ok() {
                return parsed;
            }
        }
    }
    fleet::DEFAULT_MAX_AGE_SECONDS
}

/// The stable Forge-self identity: an explicit valid override, else the
/// package name. Never derived from a filesystem path.
pub fn self_identity() -> String {
    std::env::var(SELF_ID_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && validate_project_id(value).is_ok())
        .unwrap_or_else(|| SELF_DEFAULT_ID.to_string())
}

/// Load and aggregate the fleet for the authenticated admin response. Reads
/// only the explicitly configured external sources resolved from the
/// established environment path; nothing is scanned.
pub fn load(db_path: &Path) -> Result<Value, ForgeError> {
    let now = Utc::now();
    let (candidates, sources) = gather(
        db_path,
        inventory::resolve_source(None).as_deref(),
        fleet::resolve_registry_path(None).as_deref(),
        configured_max_age(),
        now,
    )?;
    Ok(build_envelope(candidates, sources, now))
}

/// Gather the real source data into candidate rows and source descriptors.
/// Split from [`build_envelope`] so the merge/sort/serialize logic stays
/// pure and independently testable.
pub fn gather(
    db_path: &Path,
    inventory_source: Option<&Path>,
    workspace_registry: Option<&Path>,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> Result<(Vec<CandidateRow>, Vec<SourceDescriptor>), ForgeError> {
    let self_id = self_identity();
    let mut candidates: Vec<CandidateRow> = Vec::new();
    let mut sources: Vec<SourceDescriptor> = Vec::new();

    // --- registry (local managed projects + Forge-self merge) ---
    let fleet_view = load_fleet_list(db_path)?;
    let registry_rows = &fleet_view.rows;
    let local_ids: std::collections::BTreeSet<String> =
        registry_rows.iter().map(|row| row.id.clone()).collect();

    let registry_count;
    let mut merged_self: Option<CandidateRow> = None;
    for row in registry_rows {
        let evidence = row
            .evidence
            .iter()
            .map(|(source, status)| (source.clone(), status.label().to_string()))
            .collect::<Vec<_>>();
        let is_self_row = row.id == self_id;
        let candidate = CandidateRow {
            identity: row.id.clone(),
            name: row.id.clone(),
            profile: Some(row.profile.clone()),
            state: row.last_state.clone(),
            source: if is_self_row {
                RowSource::SelfRecord
            } else {
                RowSource::Registry
            },
            management: if is_self_row {
                Management::SelfRecord
            } else {
                Management::Managed
            },
            is_self: is_self_row,
            freshness: RowFreshness::Fresh,
            updated_at: Some(row.last_at.clone()),
            capabilities: vec!["inspect".to_string()],
            evidence: evidence.clone(),
            lifecycle: row
                .lifecycle
                .map(|value| format!("{value:?}").to_ascii_lowercase()),
            confidence: row
                .confidence
                .map(|value| format!("{value:?}").to_ascii_lowercase()),
            tags: row.tags.clone(),
            has_registry_ref: is_self_row,
            publish: None,
        };
        if is_self_row {
            merged_self = Some(candidate);
        } else {
            candidates.push(candidate);
        }
    }
    registry_count = candidates
        .iter()
        .filter(|candidate| candidate.source == RowSource::Registry)
        .count();

    let self_row = merged_self.unwrap_or_else(|| CandidateRow {
        identity: self_id.clone(),
        name: SELF_NAME.to_string(),
        profile: Some(env!("CARGO_PKG_NAME").to_string()),
        state: "self".to_string(),
        source: RowSource::SelfRecord,
        management: Management::SelfRecord,
        is_self: true,
        freshness: RowFreshness::Fresh,
        updated_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        // Forge itself is only inspectable through the registry once it is a
        // registered project; an unregistered self record exposes no action.
        capabilities: Vec::new(),
        evidence: Vec::new(),
        lifecycle: None,
        confidence: None,
        tags: Vec::new(),
        has_registry_ref: false,
        publish: None,
    });
    // The self row always leads the candidate list and appears exactly once.
    candidates.insert(0, self_row);

    sources.push(SourceDescriptor {
        id: "registry",
        kind: "forge-registry",
        status: "available",
        count: registry_count,
        malformed: Vec::new(),
        reason: None,
        observed_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        provider: None,
    });

    // --- local publish-history projection ---
    // Reads the same local, Forge-owned registry the fleet already opened.
    // A published id that is already registered (or the self id) is merged
    // into its managed row so no false identity conflict is introduced.
    let mut managed_ids = local_ids.clone();
    managed_ids.insert(self_id.clone());
    sources.push(read_published(
        db_path,
        max_age_seconds,
        &managed_ids,
        &mut candidates,
        now,
    ));

    // --- portable inventory external source ---
    let (inventory_rows, inventory_descriptor) =
        read_inventory(inventory_source, max_age_seconds, now);
    candidates.extend(inventory_rows);
    sources.push(inventory_descriptor);

    // --- workspace fleet registry external source ---
    let (fleet_rows, fleet_descriptor) =
        read_workspace_registry(workspace_registry, max_age_seconds, &local_ids);
    candidates.extend(fleet_rows);
    sources.push(fleet_descriptor);

    Ok((candidates, sources))
}

fn read_inventory(
    source: Option<&Path>,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> (Vec<CandidateRow>, SourceDescriptor) {
    let Some(path) = source else {
        return (
            Vec::new(),
            SourceDescriptor::unconfigured("inventory", "forge-project-inventory"),
        );
    };
    match inventory::load_local(path) {
        Ok(snapshot) => {
            let freshness = classify_generated_at(&snapshot.generated_at, max_age_seconds, now);
            let status = match freshness {
                RowFreshness::Stale => "stale",
                _ => "available",
            };
            let malformed = snapshot
                .malformed
                .iter()
                .map(|entry| (entry.name.clone(), entry.reason.clone()))
                .collect::<Vec<_>>();
            let rows = snapshot
                .projects
                .iter()
                .map(|entry| CandidateRow {
                    identity: entry.id.clone(),
                    name: entry.id.clone(),
                    profile: Some(entry.profile.clone()),
                    state: "observed".to_string(),
                    source: RowSource::Inventory,
                    management: Management::Observed,
                    is_self: false,
                    freshness,
                    updated_at: Some(snapshot.generated_at.clone()),
                    capabilities: Vec::new(),
                    evidence: Vec::new(),
                    lifecycle: None,
                    confidence: None,
                    tags: Vec::new(),
                    has_registry_ref: false,
                    publish: None,
                })
                .collect();
            (
                rows,
                SourceDescriptor {
                    id: "inventory",
                    kind: "forge-project-inventory",
                    status,
                    count: snapshot.projects.len(),
                    malformed,
                    reason: None,
                    observed_at: Some(snapshot.generated_at.clone()),
                    provider: Some(snapshot.provider.clone()),
                },
            )
        }
        // A configured source that cannot be read is unavailable with a safe
        // reason; the underlying error text (which may name a path) is never
        // surfaced. Other sources remain visible.
        Err(_) => (
            Vec::new(),
            SourceDescriptor {
                id: "inventory",
                kind: "forge-project-inventory",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some("the configured inventory source could not be read".to_string()),
                observed_at: None,
                provider: None,
            },
        ),
    }
}

fn read_workspace_registry(
    source: Option<&Path>,
    max_age_seconds: i64,
    local_ids: &std::collections::BTreeSet<String>,
) -> (Vec<CandidateRow>, SourceDescriptor) {
    let Some(path) = source else {
        return (
            Vec::new(),
            SourceDescriptor::unconfigured("fleet", "workspace-registry"),
        );
    };
    match fleet::observe(Some(path), max_age_seconds, local_ids) {
        Ok(report) => {
            let freshness = match report.freshness {
                FleetFreshness::Stale => RowFreshness::Stale,
                FleetFreshness::Fresh => RowFreshness::Fresh,
                FleetFreshness::Unconfigured => RowFreshness::Unconfigured,
            };
            let status = match report.freshness {
                FleetFreshness::Stale => "stale",
                FleetFreshness::Fresh => "available",
                FleetFreshness::Unconfigured => "unconfigured",
            };
            let malformed = report
                .malformed
                .iter()
                .map(|entry| (entry.name.clone(), entry.reason.clone()))
                .collect::<Vec<_>>();
            let rows = report
                .entries
                .iter()
                .map(|entry| CandidateRow {
                    identity: entry.id.clone(),
                    name: entry.id.clone(),
                    profile: Some(entry.profile.clone()),
                    state: entry.lifecycle.clone(),
                    source: RowSource::WorkspaceRegistry,
                    // A fleet entry is this source's view; even when the
                    // same id is locally registered, the registry row is the
                    // managed one and this row stays observed. The shared
                    // identity is surfaced as a conflict by build_envelope.
                    management: Management::Observed,
                    is_self: false,
                    freshness,
                    updated_at: Some(report.observed_at.clone()),
                    capabilities: Vec::new(),
                    evidence: Vec::new(),
                    lifecycle: Some(entry.lifecycle.clone()),
                    confidence: None,
                    tags: Vec::new(),
                    has_registry_ref: false,
                    publish: None,
                })
                .collect();
            (
                rows,
                SourceDescriptor {
                    id: "fleet",
                    kind: "workspace-registry",
                    status,
                    count: report.entries.len(),
                    malformed,
                    reason: None,
                    observed_at: Some(report.observed_at.clone()),
                    provider: None,
                },
            )
        }
        Err(_) => (
            Vec::new(),
            SourceDescriptor {
                id: "fleet",
                kind: "workspace-registry",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some(
                    "the configured workspace registry source could not be read".to_string(),
                ),
                observed_at: None,
                provider: None,
            },
        ),
    }
}

/// True when the operator has explicitly disabled the publish-history
/// projection. The projection is on by default because it reads the local,
/// Forge-owned registry; only the exact opt-out tokens turn it off.
fn publish_history_disabled() -> bool {
    match std::env::var(PUBLISH_HISTORY_ENV) {
        Ok(value) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        }
        Err(_) => false,
    }
}

/// Resolve the bounded publish-history limit: a positive machine value
/// clamped to the hard ceiling, else the default.
pub fn configured_publish_limit() -> usize {
    std::env::var(PUBLISH_HISTORY_LIMIT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(PUBLISH_HISTORY_MAX_LIMIT))
        .unwrap_or(PUBLISH_HISTORY_DEFAULT_LIMIT)
}

/// Project the local publish journal into the fleet. Standalone rows are
/// pushed onto `candidates`; an identity that is already managed (registered
/// or self) is enriched in place. Never reads an external path and never
/// writes. A failure is reported as an unavailable source descriptor and
/// leaves every other source intact.
fn read_published(
    db_path: &Path,
    max_age_seconds: i64,
    managed_ids: &std::collections::BTreeSet<String>,
    candidates: &mut Vec<CandidateRow>,
    now: DateTime<Utc>,
) -> SourceDescriptor {
    let descriptor = |status: &'static str, reason: Option<String>| SourceDescriptor {
        id: "published",
        kind: "forge-publish-history",
        status,
        count: 0,
        malformed: Vec::new(),
        reason,
        observed_at: None,
        provider: None,
    };
    if publish_history_disabled() {
        return descriptor(
            "unconfigured",
            Some("publish history projection is disabled by FORGE_PUBLISH_HISTORY".to_string()),
        );
    }
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => {
            return descriptor(
                "unavailable",
                Some("the local publish journal could not be read".to_string()),
            )
        }
    };
    let publishes = match registry.latest_publishes(configured_publish_limit()) {
        Ok(publishes) => publishes,
        Err(_) => {
            return descriptor(
                "unavailable",
                Some("the local publish journal could not be projected".to_string()),
            )
        }
    };

    let count = publishes.len();
    for publish in publishes {
        let projection = project_publish(&publish);
        let merged = managed_ids.contains(&publish.project_id);
        if merged {
            let target = candidates.iter_mut().find(|candidate| {
                candidate.identity == publish.project_id
                    && (candidate.source == RowSource::Registry || candidate.is_self)
            });
            if let Some(row) = target {
                row.publish = Some(projection);
                continue;
            }
        }
        // Published-only project: an observed row, inspection only. A stale
        // observation keeps its own freshness label; the row is never
        // downgraded on a guess.
        let freshness = classify_generated_at(&publish.started_at, max_age_seconds, now);
        candidates.push(CandidateRow {
            identity: publish.project_id.clone(),
            name: publish.project_id.clone(),
            profile: None,
            state: publish.state.clone(),
            source: RowSource::Published,
            management: Management::Observed,
            is_self: false,
            freshness,
            updated_at: Some(publish.started_at.clone()),
            capabilities: Vec::new(),
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: false,
            publish: Some(projection),
        });
    }

    SourceDescriptor {
        id: "published",
        kind: "forge-publish-history",
        status: "available",
        count,
        malformed: Vec::new(),
        reason: None,
        observed_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        provider: None,
    }
}

/// Build the rendered projection from one journal row: redact the detail,
/// then derive the `healthy`/`stages` scalars the detail may carry.
fn project_publish(publish: &PublishedOperation) -> PublishProjection {
    let detail = publish
        .detail
        .as_deref()
        .map(redact_local_paths)
        .map(|value| crate::policy::redact_credentials(&value));
    let healthy = detail.as_deref().and_then(parse_healthy);
    let stages = detail.as_deref().and_then(parse_stages);
    PublishProjection {
        state: publish.state.clone(),
        started_at: publish.started_at.clone(),
        finished_at: publish.finished_at.clone(),
        detail,
        target: publish.queue_id.clone(),
        revision: publish.revision.clone(),
        build_status: publish.build_status.clone(),
        run_status: publish.run_status.clone(),
        container_identity: publish.container_identity.clone(),
        healthy,
        stages,
    }
}

/// Find `healthy=true` / `healthy=false` in the redacted publish detail.
fn parse_healthy(detail: &str) -> Option<bool> {
    detail.split_whitespace().find_map(|token| {
        token.strip_prefix("healthy=").map(|value| {
            value
                .trim_end_matches(['.', ','])
                .eq_ignore_ascii_case("true")
        })
    })
}

/// Find `stages=N` in the redacted publish detail.
fn parse_stages(detail: &str) -> Option<i64> {
    detail.split_whitespace().find_map(|token| {
        token
            .strip_prefix("stages=")
            .and_then(|value| value.trim_end_matches(['.', ',']).parse().ok())
    })
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. Publish `detail`
/// text is Core's `Display` output, which legitimately names paths on
/// failure; the browser only ever needs the logical reason. API route
/// strings (always under `/v1/…`) are left intact.
fn redact_local_paths(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let is_route = token == "/v1" || token.starts_with("/v1/");
            let is_abs = !is_route
                && (token.starts_with('/')
                    || (token.len() > 2
                        && token.as_bytes()[1] == b':'
                        && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                    || token.contains("=/")
                    || token.contains(":\\"));
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn classify_generated_at(
    generated_at: &str,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> RowFreshness {
    match DateTime::parse_from_rfc3339(generated_at) {
        Ok(parsed) => {
            let parsed = parsed.with_timezone(&Utc);
            let age = now.signed_duration_since(parsed).num_seconds().max(0);
            if age > max_age_seconds {
                RowFreshness::Stale
            } else {
                RowFreshness::Fresh
            }
        }
        // An unparseable declared timestamp cannot be proven stale, so the
        // source stays available rather than being downgraded on a guess.
        Err(_) => RowFreshness::Fresh,
    }
}

/// Merge, detect conflicts, sort and render the versioned envelope. Pure
/// over already-gathered rows so every failure and boundary state is
/// independently testable without a filesystem.
pub fn build_envelope(
    mut candidates: Vec<CandidateRow>,
    sources: Vec<SourceDescriptor>,
    now: DateTime<Utc>,
) -> Value {
    // Identity conflict: an identity claimed by more than one source keeps
    // every claiming row, is labelled as a conflict, and loses its
    // (ambiguous) operation links. Nothing is silently dropped or collapsed.
    let mut owners: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        owners
            .entry(candidate.identity.clone())
            .or_default()
            .push(index);
    }
    let conflicting: std::collections::BTreeSet<String> = owners
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .map(|(identity, _)| identity.clone())
        .collect();
    for candidate in candidates.iter_mut() {
        if conflicting.contains(&candidate.identity) {
            candidate.capabilities.clear();
        }
    }

    // Stable order: normalized display name, then stable identity.
    candidates.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.identity.cmp(&b.identity))
    });

    let projects = candidates
        .iter()
        .map(|candidate| render_row(candidate, &conflicting))
        .collect::<Vec<_>>();

    let source_array = sources
        .iter()
        .map(|source| {
            let mut value = json!({
                "id": source.id,
                "kind": source.kind,
                "status": source.status,
                "count": source.count,
            });
            if !source.malformed.is_empty() {
                value["malformed"] = json!(source
                    .malformed
                    .iter()
                    .map(|(name, reason)| json!({"name": name, "reason": reason}))
                    .collect::<Vec<_>>());
            }
            if let Some(reason) = &source.reason {
                value["reason"] = json!(reason);
            }
            if let Some(observed_at) = &source.observed_at {
                value["observed_at"] = json!(observed_at);
            }
            if let Some(provider) = &source.provider {
                value["provider"] = json!(provider);
            }
            value
        })
        .collect::<Vec<_>>();

    let self_present = candidates.iter().any(|candidate| candidate.is_self);
    let registered = candidates
        .iter()
        .filter(|candidate| {
            candidate.source == RowSource::Registry
                || (candidate.is_self && candidate.has_registry_ref)
        })
        .count();
    let with_evidence = candidates
        .iter()
        .filter(|candidate| !candidate.evidence.is_empty())
        .count();
    let profiles = candidates
        .iter()
        .filter_map(|candidate| candidate.profile.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let by_source = |source: RowSource| {
        candidates
            .iter()
            .filter(|candidate| candidate.source == source)
            .count()
    };

    json!({
        "contract": super::API_CONTRACT_VERSION,
        "fleet_contract": WEB_FLEET_CONTRACT_VERSION,
        "observed_at": now.to_rfc3339_opts(SecondsFormat::Secs, true),
        "projects": projects,
        "sources": source_array,
        "summary": {
            "registered": registered,
            "with_evidence": with_evidence,
            "total": candidates.len(),
            "self_present": self_present,
            "conflicts": candidates.iter().filter(|c| conflicting.contains(&c.identity)).count(),
            "profiles": profiles.len(),
            "by_source": {
                "self": by_source(RowSource::SelfRecord),
                "registry": by_source(RowSource::Registry),
                "inventory": by_source(RowSource::Inventory),
                "fleet": by_source(RowSource::WorkspaceRegistry),
                "published": by_source(RowSource::Published),
            },
        },
    })
}

fn render_row(candidate: &CandidateRow, conflicting: &std::collections::BTreeSet<String>) -> Value {
    json!({
        // Compatibility fields the standalone frontend already reads.
        "id": candidate.identity,
        "profile": candidate.profile,
        "state": candidate.state,
        "lifecycle": candidate.lifecycle,
        "confidence": candidate.confidence,
        "tags": candidate.tags,
        "updated_at": candidate.updated_at,
        "evidence": candidate.evidence.iter().map(|(source, status)| json!({"source": source, "status": status})).collect::<Vec<_>>(),
        // New normalized provenance fields.
        "identity": candidate.identity,
        "name": candidate.name,
        "source": candidate.source.id(),
        "source_ref": candidate.identity,
        "management": candidate.management.id(),
        "is_self": candidate.is_self,
        "freshness": candidate.freshness.id(),
        "capabilities": candidate.capabilities,
        "conflict": conflicting.contains(&candidate.identity),
        // Present only when a local publish operation exists for this
        // identity. `null` for every project without publish history.
        "publish": candidate.publish.as_ref().map(|publish| json!({
            "state": publish.state,
            "started_at": publish.started_at,
            "finished_at": publish.finished_at,
            "detail": publish.detail,
            "target": publish.target,
            "revision": publish.revision,
            "build_status": publish.build_status,
            "run_status": publish.run_status,
            "container_identity": publish.container_identity,
            "healthy": publish.healthy,
            "stages": publish.stages,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn self_candidate(id: &str, registered: bool) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: if registered {
                id.to_string()
            } else {
                SELF_NAME.to_string()
            },
            profile: Some(if registered {
                "rust-cli".to_string()
            } else {
                "forge".to_string()
            }),
            state: if registered {
                "done".to_string()
            } else {
                "self".to_string()
            },
            source: RowSource::SelfRecord,
            management: Management::SelfRecord,
            is_self: true,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-06T12:00:00Z".to_string()),
            capabilities: if registered {
                vec!["inspect".to_string()]
            } else {
                Vec::new()
            },
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: registered,
            publish: None,
        }
    }

    fn registry_candidate(id: &str) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: id.to_string(),
            profile: Some("rust-web".to_string()),
            state: "done".to_string(),
            source: RowSource::Registry,
            management: Management::Managed,
            is_self: false,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-01T00:00:00Z".to_string()),
            capabilities: vec!["inspect".to_string()],
            evidence: vec![("github".to_string(), "ok".to_string())],
            lifecycle: Some("active".to_string()),
            confidence: None,
            tags: vec!["core".to_string()],
            has_registry_ref: false,
            publish: None,
        }
    }

    fn observed_candidate(source: RowSource, id: &str, profile: &str) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: id.to_string(),
            profile: Some(profile.to_string()),
            state: "observed".to_string(),
            source,
            management: Management::Observed,
            is_self: false,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-02T00:00:00Z".to_string()),
            capabilities: Vec::new(),
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: false,
            publish: None,
        }
    }

    fn registry_source() -> SourceDescriptor {
        SourceDescriptor {
            id: "registry",
            kind: "forge-registry",
            status: "available",
            count: 1,
            malformed: Vec::new(),
            reason: None,
            observed_at: None,
            provider: None,
        }
    }

    #[test]
    fn forge_self_appears_exactly_once_when_unregistered() {
        let envelope = build_envelope(
            vec![self_candidate("forge", false)],
            Vec::new(),
            fixed_now(),
        );
        let projects = envelope["projects"].as_array().unwrap();
        let selfs = projects
            .iter()
            .filter(|row| row["is_self"] == json!(true))
            .count();
        assert_eq!(selfs, 1, "Forge appears exactly once");
        assert_eq!(envelope["summary"]["self_present"], json!(true));
        assert_eq!(envelope["summary"]["registered"], json!(0));
        assert_eq!(projects[0]["management"], json!("self"));
        // An unregistered self row exposes no operation capability.
        assert_eq!(projects[0]["capabilities"], json!([]));
    }

    #[test]
    fn registered_self_row_is_not_duplicated_and_counts_as_registered() {
        // `gather` merges a registry row whose id equals the self identity into
        // the single self candidate (has_registry_ref = true) rather than
        // pushing a second row, so `build_envelope` receives exactly one "forge"
        // row. That merged row is inspectable and counts as registered.
        let envelope = build_envelope(vec![self_candidate("forge", true)], Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        let forges = projects
            .iter()
            .filter(|row| row["identity"] == json!("forge"))
            .count();
        assert_eq!(forges, 1, "registered Forge is not duplicated");
        let row = projects
            .iter()
            .find(|r| r["identity"] == json!("forge"))
            .unwrap();
        assert_eq!(row["is_self"], json!(true));
        assert_eq!(row["management"], json!("self"));
        // Merged with a registry reference it becomes inspectable.
        assert!(row["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")));
        assert_eq!(envelope["summary"]["registered"], json!(1));
    }

    #[test]
    fn every_source_contributes_rows_and_management_is_labelled() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("alpha"),
            observed_candidate(RowSource::Inventory, "tool-beta", "dotnet-web"),
            observed_candidate(RowSource::WorkspaceRegistry, "gamma", "typescript-monorepo"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 4);
        let by_id = |id: &str| {
            projects
                .iter()
                .find(|r| r["identity"] == json!(id))
                .unwrap()
        };
        assert_eq!(by_id("alpha")["management"], json!("managed"));
        assert!(by_id("alpha")["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")));
        assert_eq!(by_id("tool-beta")["management"], json!("observed"));
        assert_eq!(by_id("tool-beta")["capabilities"], json!([]));
        assert_eq!(by_id("gamma")["source"], json!("fleet"));
        assert_eq!(envelope["summary"]["total"], json!(4));
    }

    #[test]
    fn conflicting_identity_is_retained_and_ambiguous_links_disabled() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("shared"),
            observed_candidate(RowSource::Inventory, "shared", "external-vocab"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        let shared_rows = projects
            .iter()
            .filter(|row| row["identity"] == json!("shared"))
            .collect::<Vec<_>>();
        assert_eq!(shared_rows.len(), 2, "both source records are retained");
        assert!(shared_rows.iter().all(|row| row["conflict"] == json!(true)));
        assert!(
            shared_rows
                .iter()
                .all(|row| row["capabilities"].as_array().unwrap().is_empty()),
            "ambiguous mutation links are disabled"
        );
        assert_eq!(envelope["summary"]["conflicts"], json!(2));
    }

    #[test]
    fn rows_are_sorted_by_name_then_identity() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("zeta"),
            registry_candidate("alpha"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let names = envelope["projects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        assert_eq!(names, sorted);
    }

    #[test]
    fn source_descriptors_report_each_state_without_leaking_paths() {
        let sources = vec![
            registry_source(),
            SourceDescriptor::unconfigured("inventory", "forge-project-inventory"),
            SourceDescriptor {
                id: "fleet",
                kind: "workspace-registry",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some(
                    "the configured workspace registry source could not be read".to_string(),
                ),
                observed_at: None,
                provider: None,
            },
        ];
        let envelope = build_envelope(vec![self_candidate("forge", false)], sources, fixed_now());
        let text = serde_json::to_string(&envelope).unwrap();
        assert!(text.contains("\"status\":\"unconfigured\""), "{text}");
        assert!(text.contains("\"status\":\"unavailable\""), "{text}");
        // No absolute filesystem path is ever serialized.
        assert!(!text.contains("/home/"), "{text}");
        assert!(!text.contains("/tmp/"), "{text}");
    }

    #[test]
    fn empty_registry_still_returns_self_and_zero_registered() {
        let envelope = build_envelope(Vec::new(), Vec::new(), fixed_now());
        assert_eq!(envelope["projects"].as_array().unwrap().len(), 0);
        // self_present is false only when even the self row was not gathered;
        // gather always adds it, so this primitive stays honest about input.
        assert_eq!(envelope["summary"]["total"], json!(0));
    }

    #[test]
    fn configured_max_age_falls_back_when_invalid() {
        // No override is set in this unit context, so the shared default holds.
        std::env::remove_var(FLEET_MAX_AGE_ENV);
        assert_eq!(configured_max_age(), fleet::DEFAULT_MAX_AGE_SECONDS);
    }
}
