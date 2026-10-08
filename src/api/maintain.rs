//! Per-project maintainer surface (`forge-project-maintain/0.1.0`).
//!
//! One read-only route assembles what the browser's Maintain card
//! renders — the GitHub observation as Forge last saw it (with its
//! freshness, or an honest `unavailable` with the reason), the
//! derived classification proposals, and the configured plugin
//! registry — plus three preview → confirm → apply routes for the
//! per-field approve/reject and the publish action.
//!
//! Boundary: the browser addresses a project only by its validated
//! id; the root is resolved server-side from the registry. No
//! filesystem path, argv or credential ever crosses the wire.

use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{ApiRequest, ApiResponse};
use crate::catalog;
use crate::core::manifest::Manifest;
use crate::github::{self, GithubObservation, GithubState};
use crate::plugins;
use crate::publish::providers;
use crate::registry::Registry;
use crate::semantic::{self, ProposalId, ProposalState};

/// Versioned contract for the maintainer surface.
pub const CONTRACT_VERSION: &str = "forge-project-maintain/0.1.0";

/// `GET /v1/admin/projects/{id}/maintain` — read-only, bounded.
pub const ROUTE_PROJECT_MAINTAIN: &str = "GET /v1/admin/projects/{id}/maintain";
/// `POST /v1/admin/projects/{id}/classify/approve` — preview + confirm.
pub const ROUTE_CLASSIFY_APPROVE: &str = "POST /v1/admin/projects/{id}/classify/approve";
/// `POST /v1/admin/projects/{id}/classify/reject` — preview + confirm.
pub const ROUTE_CLASSIFY_REJECT: &str = "POST /v1/admin/projects/{id}/classify/reject";
/// `POST /v1/admin/projects/{id}/classify/apply` — preview + confirm.
pub const ROUTE_CLASSIFY_APPLY: &str = "POST /v1/admin/projects/{id}/classify/apply";

/// Upper bound on proposals echoed to the browser, newest first.
const MAX_PROPOSALS: usize = 50;

/// The read-only maintainer projection for one managed project.
pub fn maintain(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match super::workbench::resolve(&registry, id) {
        super::workbench::Resolved::Managed(record) => record,
        super::workbench::Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    let project_dir = Path::new(&record.path);

    let github = github_observation(&record);
    let proposals = proposals(project_dir);
    let plugins = plugin_records(project_dir);

    ApiResponse::json(
        200,
        json!({
            "contract": CONTRACT_VERSION,
            "project_id": record.id,
            "github": github,
            "proposals": proposals,
            "plugins": plugins,
        }),
    )
}

/// The GitHub observation as Forge last saw it. An unreachable
/// remote renders `unavailable` with the reason — a blank field and
/// an unreachable field are different facts.
fn github_observation(record: &crate::registry::ProjectRecord) -> Value {
    let repository = record
        .git_remote
        .as_deref()
        .and_then(catalog::source::github_repository_from_remote);
    let Some(repository) = repository else {
        return json!({
            "state": "unavailable",
            "reason": "no GitHub remote is declared for this project",
            "observed_at": null,
            "freshness": "unknown",
        });
    };
    let adapter = github::GithubAdapter::from_env();
    if !adapter.binary_available() {
        return json!({
            "state": "unavailable",
            "reason": format!("GitHub adapter binary is not configured ({})", adapter.source),
            "observed_at": null,
            "freshness": "unknown",
        });
    }
    if adapter.token.is_none() {
        return json!({
            "state": "unavailable",
            "reason": format!("{} is not set; the GitHub observation cannot be made", github::GITHUB_TOKEN_ENV),
            "observed_at": null,
            "freshness": "unknown",
        });
    }
    let request = github::GithubObservationRequest {
        host: github::GITHUB_DEFAULT_HOST.to_string(),
        repositories: vec![repository],
    };
    match adapter.observe(&request) {
        Ok(observations) => match observations.into_iter().next() {
            Some(observation) => observation_json(&observation),
            None => json!({
                "state": "unavailable",
                "reason": "the GitHub adapter returned no observation",
                "observed_at": null,
                "freshness": "unknown",
            }),
        },
        Err(err) => json!({
            "state": "unavailable",
            "reason": err.to_string(),
            "observed_at": null,
            "freshness": "unknown",
        }),
    }
}

/// Project one observation into the card's shape. `current` and
/// `stale` carry the observed fields; every other state carries the
/// reason and never a fabricated empty field.
fn observation_json(observation: &GithubObservation) -> Value {
    let now = Utc::now();
    let freshness = catalog::record::freshness_from(
        &observation.observed_at,
        catalog::DEFAULT_MAX_AGE_SECONDS,
        now,
    );
    match &observation.state {
        GithubState::Current | GithubState::Stale => {
            let homepage = observation
                .custom_properties
                .iter()
                .find(|(key, _)| key == "homepage")
                .map(|(_, value)| value.clone());
            json!({
                "state": observation.state.id(),
                "observed_at": observation.observed_at,
                "freshness": freshness.id(),
                "source_revision": observation.source_revision,
                "description": observation.description,
                "topics": observation.topics,
                "homepage": homepage,
                "language": observation.languages.first().cloned(),
                "languages": observation.languages,
            })
        }
        other => json!({
            "state": other.id(),
            "reason": reason_for(other),
            "observed_at": observation.observed_at,
            "freshness": freshness.id(),
        }),
    }
}

fn reason_for(state: &GithubState) -> String {
    match state {
        GithubState::Unavailable { reason } => reason.clone(),
        GithubState::RateLimited { reset_at } => {
            format!("rate limited; resets at {reset_at}")
        }
        GithubState::Unauthorized => "the GitHub token is not configured".to_string(),
        GithubState::Forbidden => "the GitHub token cannot read this repository".to_string(),
        GithubState::NotFound => "the repository does not exist".to_string(),
        GithubState::Partial => "the adapter reported a partial observation".to_string(),
        GithubState::Current | GithubState::Stale => String::new(),
    }
}

/// Derived classification proposals, newest first, bounded.
fn proposals(project_dir: &Path) -> Vec<Value> {
    let entries = match semantic::list(project_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .iter()
        .take(MAX_PROPOSALS)
        .map(|entry| {
            json!({
                "id": entry.dir_name,
                "kind": entry.kind.label(),
                "state": entry.state.label(),
                "confidence": entry.confidence.label(),
                "provider": entry.provider.label(),
                "evidence_count": entry.evidence_count,
                "suggested_at": entry.suggested_at,
                "decided_at": entry.decided_at,
            })
        })
        .collect()
}

/// The configured plugin registry for this project, bounded.
fn plugin_records(project_dir: &Path) -> Vec<Value> {
    let config_path = plugins::resolve_config_path(Some(project_dir));
    let (provider_config, descriptors) = if config_path.is_file() {
        let provider_config = match providers::load_config(&config_path) {
            Ok(config) => config,
            Err(_) => return Vec::new(),
        };
        let descriptors = match plugins::load_descriptors(&config_path) {
            Ok(descriptors) => descriptors,
            Err(_) => return Vec::new(),
        };
        (provider_config, descriptors)
    } else {
        (
            providers::ProviderConfig::default(),
            plugins::PluginConfig::default(),
        )
    };
    plugins::list(&provider_config, &descriptors)
        .iter()
        .map(|record| {
            json!({
                "id": record.id,
                "kind": record.kind.as_str(),
                "enabled": record.enabled,
                "capabilities": record.capability_list(),
                "state": record.state.as_str(),
                "reason": record.state.reason(),
            })
        })
        .collect()
}

/// `POST .../classify/approve` — preview, then confirm- and digest-bound
/// approve of one proposal.
pub fn classify_approve(
    db_path: &Path,
    id: &str,
    body: &Value,
    request: &ApiRequest,
) -> ApiResponse {
    classify_decide(db_path, id, body, request, true)
}

/// `POST .../classify/reject` — preview, then confirm- and digest-bound
/// reject of one proposal.
pub fn classify_reject(
    db_path: &Path,
    id: &str,
    body: &Value,
    request: &ApiRequest,
) -> ApiResponse {
    classify_decide(db_path, id, body, request, false)
}

fn classify_decide(
    db_path: &Path,
    id: &str,
    body: &Value,
    _request: &ApiRequest,
    approve: bool,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match super::workbench::resolve(&registry, id) {
        super::workbench::Resolved::Managed(record) => record,
        super::workbench::Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    let project_dir = Path::new(&record.path);
    let proposal_raw = match body.get("proposal").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            return refuse(
                400,
                "a `proposal` id (`<kind>-<hash>`) is required; nothing was changed.",
            )
        }
    };
    let proposal_id = match parse_proposal_id(project_dir, proposal_raw) {
        Ok(id) => id,
        Err(err) => return typed_refusal(&err),
    };
    let current =
        match semantic::read(project_dir, &proposal_id) {
            Ok(Some(proposal)) => proposal,
            Ok(None) => return refuse(
                404,
                "that proposal was not found under this project's `.forge/semantic/` directory.",
            ),
            Err(err) => return typed_refusal(&err),
        };
    let action = if approve { "approve" } else { "reject" };
    let preview = json!({
        "action": action,
        "proposal": proposal_raw,
        "kind": current.kind.label(),
        "current_state": current.state.label(),
        "confidence": current.confidence.label(),
    });
    let digest = digest_of(&preview);
    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "effect": "none",
                "preview": preview,
                "plan_digest": digest,
                "confirmation": {
                    "requires": ["confirm", "plan_digest", "proposal"],
                    "note": "This preview writes nothing. Confirm by sending `confirm: true` with this exact `plan_digest` and the same `proposal`; a changed or stale digest is refused.",
                },
            }),
        );
    }
    let supplied = body
        .get("plan_digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if supplied != digest {
        return ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "error": {
                    "code": "stale-plan-digest",
                    "message": "the confirmed digest does not match this proposal's current preview; nothing was changed. Review the refreshed preview and confirm its new digest.",
                },
                "preview": preview,
                "plan_digest": digest,
            }),
        );
    }
    let outcome = if approve {
        semantic::approve(project_dir, &proposal_id, Utc::now(), true)
    } else {
        semantic::reject(project_dir, &proposal_id, Utc::now(), true)
    };
    match outcome {
        Ok(outcome) => ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "accepted": true,
                "proposal": proposal_raw,
                "state": outcome.state.label(),
                "note": outcome.note,
            }),
        ),
        Err(err) => typed_refusal(&err),
    }
}

/// `POST .../classify/apply` — preview, then confirm- and digest-bound
/// apply of the approved set through the configured metadata plugin.
pub fn classify_apply(db_path: &Path, id: &str, body: &Value, request: &ApiRequest) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match super::workbench::resolve(&registry, id) {
        super::workbench::Resolved::Managed(record) => record,
        super::workbench::Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    let project_dir = Path::new(&record.path);

    // An idempotent retry of a confirmed apply returns the journaled
    // operation rather than re-running the plugin.
    if let Some(key) = request.idempotency_key.as_deref() {
        if let Ok(Some(entry)) = registry.operation_by_idempotency("classify.apply", key) {
            if entry.project_id == id {
                let status = if entry.state == "pending" { 202 } else { 200 };
                return ApiResponse::json(
                    status,
                    json!({
                        "contract": CONTRACT_VERSION,
                        "accepted": true,
                        "replay": true,
                        "operation_id": entry.op_id,
                        "project_id": id,
                        "state": entry.state,
                        "started_at": entry.started_at,
                        "finished_at": entry.finished_at,
                        "detail": entry.detail.as_deref().map(super::workbench::redact_local_paths),
                    }),
                );
            }
        }
    }

    let entries = match semantic::list(project_dir) {
        Ok(entries) => entries,
        Err(err) => return typed_refusal(&err),
    };
    let approved: Vec<&semantic::ProposalListEntry> = entries
        .iter()
        .filter(|entry| entry.state == ProposalState::Approved)
        .collect();
    let preview = json!({
        "approved_proposals": approved.iter().map(|entry| entry.dir_name.clone()).collect::<Vec<_>>(),
        "fields": approved.iter().map(|entry| entry.kind.label()).collect::<Vec<_>>(),
        "mode": "pr",
    });
    let digest = digest_of(&preview);
    let confirm = body
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            200,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "effect": "none",
                "preview": preview,
                "plan_digest": digest,
                "confirmation": {
                    "requires": ["confirm", "plan_digest"],
                    "note": "This preview sends nothing to any plugin. Confirm by sending `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
                },
            }),
        );
    }
    let supplied = body.get("plan_digest").and_then(Value::as_str);
    if supplied != Some(digest.as_str()) {
        return ApiResponse::json(
            409,
            json!({
                "contract": CONTRACT_VERSION,
                "project_id": record.id,
                "error": {
                    "code": "stale-plan-digest",
                    "preview": preview,
                    "plan_digest": digest,
                },
                "preview": preview,
                "plan_digest": digest,
            }),
        );
    }
    match semantic::apply(project_dir, true) {
        Ok(outcome) => {
            let _ = registry.record_operation(
                "classify.apply",
                &record.id,
                "succeeded",
                &format!(
                    "plugin={} fields={} pr={}",
                    outcome.plugin_id,
                    outcome.fields.join(","),
                    outcome.pr_reference.as_deref().unwrap_or("unknown")
                ),
            );
            ApiResponse::json(
                200,
                json!({
                    "contract": CONTRACT_VERSION,
                    "project_id": record.id,
                    "accepted": true,
                    "plugin_id": outcome.plugin_id,
                    "fields": outcome.fields,
                    "pr_reference": outcome.pr_reference,
                    "note": outcome.note,
                }),
            )
        }
        Err(err) => typed_refusal(&err),
    }
}

/// Parse `<kind>-<hash>` into a [`ProposalId`], mirroring the CLI's
/// `parse_proposal_id`.
fn parse_proposal_id(project_dir: &Path, raw: &str) -> Result<ProposalId, crate::core::ForgeError> {
    let (kind_label, hash) = raw.rsplit_once('-').ok_or_else(|| {
        crate::core::ForgeError::SemanticInvalid {
            reason: format!(
                "proposal id `{raw}` is malformed: expected `<kind>-<hash>` (e.g. `description-deadbeef`)"
            ),
        }
    })?;
    let kind = semantic::parse_kind(kind_label)?;
    let (manifest, _) = Manifest::load_from_dir(project_dir, None).map_err(|err| {
        crate::core::ForgeError::SemanticInvalid {
            reason: format!("cannot resolve project id: {err}"),
        }
    })?;
    Ok(ProposalId {
        project_id: manifest.project.id,
        kind,
        hash: hash.to_string(),
    })
}

fn digest_of(preview: &Value) -> String {
    let bytes = serde_json::to_vec(preview).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for byte in out {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn unavailable() -> ApiResponse {
    ApiResponse::json(
        503,
        json!({
            "error": {
                "code": "registry-unavailable",
                "message": "the project registry is unavailable; no maintainer data can be served.",
            }
        }),
    )
}

fn refuse(status: u16, reason: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": "maintain-refused",
                "message": reason,
            }
        }),
    )
}

fn typed_refusal(err: &crate::core::ForgeError) -> ApiResponse {
    ApiResponse::from_error(err)
}
