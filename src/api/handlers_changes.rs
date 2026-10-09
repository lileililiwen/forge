//! Project mutation handlers (`handle_doctor`, `handle_governance`,
//! `handle_create_project`, feature/upgrade/spec/agent transitions).
//!
//! Typed HTTP handlers for the mutating `/v1/projects/{id}/*` surface.
//! Bodies moved verbatim from `src/api/mod.rs` by splitrs and sub-split
//! by route family.
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::agent::{
    apply_transition as apply_agent_transition, new_session, read_session, AgentProvider,
    SessionTransition,
};
use crate::core::ForgeError;
use crate::doctor::{
    parse_target_level, run_doctor, FindingStatus, RegistryObservation, Remediation,
};
use crate::feature::{add_feature, remove_feature, upgrade_feature};
use crate::generate::{generate, normalize_explicit, GeneratedProject};
use crate::policy::{run_driftwatch, DriftWatchConfig, PolicyFinding, PolicySeverity};
use crate::registry::Registry;
use crate::spec::{
    apply_routing, ensure_single_project, generate_spec, DoctorFindingInput, FindingSource,
    SpecRequest,
};
use crate::upgrade::{apply_upgrade, plan_upgrade, SemanticConflict, UpgradeOutcome};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::contract::{API_CONTRACT_VERSION, API_SYNTHETIC_PROJECT};
use super::model::{ApiRequest, ApiResponse};
use super::router::bad_request;
use super::server::run_with_operation;

pub(super) fn handle_doctor(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = PathBuf::from(&record.path);
    let level = match request.json_body().get("target").and_then(|v| v.as_str()) {
        Some(raw) => match parse_target_level(raw) {
            Ok(value) => Some(value),
            Err(err) => return ApiResponse::from_error(&err),
        },
        None => None,
    };
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at.clone()),
    });
    let policy_outcome = run_driftwatch(&project_dir, &DriftWatchConfig::from_env());
    match run_doctor(
        &project_dir,
        level,
        observation.as_ref(),
        Some(&policy_outcome),
    ) {
        Ok(report) => ApiResponse::json(
            200,
            serde_json::json!({
                "doctor": report,
                "policy": serde_json::to_value(&policy_outcome).unwrap_or(Value::Null),
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_governance(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match crate::governance::evaluate_project(Path::new(&record.path)) {
        Ok(observation) => ApiResponse::json(
            200,
            serde_json::json!({
                "governance": observation,
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_create_project(
    db_path: &Path,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let path = match body.get("path").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires a `path` field"),
    };
    let profile = match body.get("profile").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires a `profile` field"),
    };
    let id = match body.get("id").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("create_project requires an `id` field"),
    };
    let name = body.get("name").and_then(|v| v.as_str());
    let features: Vec<String> = body
        .get("features")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if let Err(reason) = crate::core::validate_project_id(id) {
        return bad_request(&format!("invalid project id: {reason}"));
    }
    let destination = PathBuf::from(path);
    let normalized =
        match normalize_explicit(Some(profile), Some(id), name, &features, &destination, None) {
            Ok(value) => value,
            Err(err) => return ApiResponse::from_error(&err),
        };
    let (created, _op_id, project_id) = match run_with_operation(
        db_path,
        "api.create_project",
        API_SYNTHETIC_PROJECT,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let generated: GeneratedProject = generate(&mut registry, &normalized)?;
            let value = serde_json::to_value(&generated).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            let project_id = generated.record.id.clone();
            let detail = format!(
                "api create_project `{}` ({}) from {}@{}",
                generated.record.id,
                generated.record.path,
                profile,
                crate::generate::GENERATOR_VERSION
            );
            let _ = registry.record_operation("api", &project_id, "done", &detail);
            Ok((value, op_id, project_id))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "created": created,
            "contract": API_CONTRACT_VERSION,
            "project_id": project_id,
        }),
    )
}

pub(in crate::api) fn handle_add_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("add_feature requires a `feature` field"),
    };
    let version = body.get("version").and_then(|v| v.as_str());
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.add_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = add_feature(&mut registry, id, feature, version)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(super) fn handle_upgrade(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = body.get("feature").and_then(|v| v.as_str());
    let confirm = body
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !confirm {
        return ApiResponse::json(
            409,
            serde_json::json!({
                "error": {
                    "code": "api-confirm-required",
                    "message": "upgrade requires `confirm: true`; refusing implicit project mutation"
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let dry_run = body
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.upgrade", id, request, |op_id, _registry| {
            if dry_run {
                let registry = Registry::open(db_path)?;
                let plan = plan_upgrade(&registry, id, feature)?;
                let value = serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
                return Ok((value, op_id, id.to_string()));
            }
            let mut registry = Registry::open(db_path)?;
            let outcome: UpgradeOutcome = apply_upgrade(&mut registry, id, feature)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "upgrade": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(in crate::api) fn handle_generate_spec(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let findings: Vec<String> = body
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if findings.is_empty() {
        return bad_request("generate_spec requires at least one finding id");
    }
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    let project_dir = match Registry::open(db_path)
        .ok()
        .and_then(|registry| registry.inspect(id).ok())
        .map(|record| PathBuf::from(record.path))
    {
        Some(value) => value,
        None => {
            return ApiResponse::from_error(&ForgeError::UnknownProject {
                query: id.to_string(),
            });
        }
    };
    let spec_request = SpecRequest {
        project_path: project_dir,
        finding_ids: findings,
        reason,
    };
    if let Err(err) = ensure_single_project(&spec_request) {
        return ApiResponse::from_error(&err);
    }
    let sources: Vec<FindingSource> = Vec::new();
    let now = Utc::now();
    let (value, _op_id, _pid) = match run_with_operation(
        db_path,
        "api.generate_spec",
        id,
        request,
        |op_id, _registry| {
            let outcome = generate_spec(&spec_request, &sources, now)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "spec": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

/// Synthesize the in-process [`FindingSource`] for `forge spec apply` exactly
/// as the CLI's `finding_source_for` does: it is derived solely from the
/// finding-name prefix (a policy, semantic-conflict or doctor finding) and the
/// server-resolved project directory — never from a caller-supplied path,
/// argv or shell text. This keeps the portal route inside the same typed,
/// id-scoped boundary the CLI runs.
fn synthesize_finding_source(
    target: &str,
    project_path: &Path,
    finding: &str,
) -> Result<FindingSource, ForgeError> {
    if let Some(stripped) = finding.strip_prefix("driftwatch-") {
        return Ok(FindingSource::Policy(PolicyFinding {
            id: stripped.to_string(),
            category: "spec".to_string(),
            severity: PolicySeverity::Fail,
            applicable: true,
            message: format!("policy finding `{stripped}`"),
            evidence: Vec::new(),
            reason: None,
        }));
    }
    if let Some(stripped) = finding.strip_prefix("semantic-") {
        let (manifest, _) = crate::core::manifest::Manifest::load_from_dir(project_path, None)?;
        return Ok(FindingSource::Conflict(SemanticConflict {
            project_id: manifest.project.id,
            feature: stripped.to_string(),
            owned_file: format!(".forge/features/{stripped}.receipt"),
            reason: "drifted receipt reported by the portal".to_string(),
            suggested_spec: format!("forge spec generate --project {target} --finding {finding}"),
        }));
    }
    Ok(FindingSource::Doctor(DoctorFindingInput {
        id: finding.to_string(),
        status: FindingStatus::Fail,
        remediation: Remediation::Manual,
        category: "spec".to_string(),
        detail: format!("finding `{finding}` routed by `forge spec apply`"),
    }))
}

pub(in crate::api) fn handle_remove_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return bad_request("remove_feature requires a `feature` field"),
    };
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.remove_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = remove_feature(&mut registry, id, feature)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(in crate::api) fn handle_upgrade_feature(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let feature = match body.get("feature").and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => value,
        _ => return bad_request("upgrade_feature requires a `feature` field"),
    };
    let version = body.get("version").and_then(|v| v.as_str());
    let (outcome, op_id, _pid) = match run_with_operation(
        db_path,
        "api.upgrade_feature",
        id,
        request,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let outcome = upgrade_feature(&mut registry, id, feature, version)?;
            let value = serde_json::to_value(&outcome).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "feature": outcome,
            "operation_id": op_id,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(in crate::api) fn handle_apply_spec(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let findings: Vec<String> = body
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if findings.is_empty() {
        return bad_request("apply_spec requires at least one finding id");
    }
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    // Resolve the stored project directory from the validated id only; the
    // browser never sends a path. A Core error carrying a path is scrubbed by
    // `ApiResponse::from_error`.
    let project_dir = match Registry::open(db_path)
        .ok()
        .and_then(|registry| registry.inspect(id).ok())
        .map(|record| PathBuf::from(record.path))
    {
        Some(value) => value,
        None => {
            return ApiResponse::from_error(&ForgeError::UnknownProject {
                query: id.to_string(),
            });
        }
    };
    let now = Utc::now();
    let (value, _op_id, _pid) = match run_with_operation(
        db_path,
        "api.apply_spec",
        id,
        request,
        |op_id, _registry| {
            // Mirror the CLI `spec apply`: route each finding individually
            // through `apply_routing`, synthesizing the same in-process
            // finding source the CLI derives from the finding-name prefix.
            let mut outcomes = Vec::new();
            for finding in &findings {
                let spec_request = SpecRequest {
                    project_path: project_dir.clone(),
                    finding_ids: vec![finding.clone()],
                    reason: reason.clone(),
                };
                ensure_single_project(&spec_request)?;
                let source = synthesize_finding_source(id, &project_dir, finding)?;
                let outcome = apply_routing(&spec_request, &source, now)?;
                outcomes.push(serde_json::to_value(&outcome).map_err(|err| {
                    ForgeError::Registry {
                        reason: err.to_string(),
                    }
                })?);
            }
            let value = Value::Array(outcomes);
            Ok((value, op_id, id.to_string()))
        },
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    ApiResponse::json(
        202,
        serde_json::json!({
            "spec": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}

pub(super) fn handle_agent_transition(
    db_path: &Path,
    id: &str,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let session_id = match body.get("session").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("agents requires a `session` field"),
    };
    let transition = match body.get("transition").and_then(|v| v.as_str()) {
        Some(value) => value,
        None => return bad_request("agents requires a `transition` field"),
    };
    let provider = body
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("opencode");
    let spec = body.get("spec").and_then(|v| v.as_str()).map(String::from);
    let provider_id = match provider {
        "opencode" => AgentProvider::Opencode,
        "codex" => AgentProvider::Codex,
        "ariadex" => AgentProvider::Ariadex,
        other => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "api-invalid",
                        "message": format!("unknown agent provider `{other}`; expected one of: opencode, codex, ariadex")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let transition_kind = match transition {
        "start" => SessionTransition::Start,
        "pause" => SessionTransition::Pause,
        "takeover" => SessionTransition::Takeover,
        "resume" => SessionTransition::Resume,
        "restart" => SessionTransition::Restart,
        "new_session" => SessionTransition::NewSession,
        other => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "api-invalid",
                        "message": format!("unknown agent transition `{other}`; expected one of: start, pause, takeover, resume, restart, new_session")
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_dir = match registry.inspect(id).map(|r| PathBuf::from(r.path)) {
        Ok(value) => value,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let (value, _op_id, _pid) =
        match run_with_operation(db_path, "api.agent", id, request, |op_id, _registry| {
            let session = match transition_kind {
                SessionTransition::Start => new_session(
                    &project_dir,
                    session_id,
                    provider_id,
                    spec.as_deref().unwrap_or(""),
                    now,
                )?,
                _ => match read_session(&project_dir, session_id)? {
                    Some(value) => value,
                    None => {
                        return Err(ForgeError::AgentUnavailable {
                            reason: format!(
                                "session `{session_id}` was not found under `.forge/agents/`"
                            ),
                        });
                    }
                },
            };
            let outcome = apply_agent_transition(session, transition_kind, now)?;
            let files = crate::agent::write_session(&project_dir, &outcome.session)?;
            let detail = format!(
                "api agent `{id}` session `{session_id}` {transition} -> `{}`",
                outcome.state.label()
            );
            let _ = registry.record_operation("api", id, "done", &detail);
            let value = serde_json::json!({
                "transition": {
                    "session_id": outcome.session.session_id,
                    "state": outcome.state.label(),
                    "requested": outcome.requested.label(),
                    "evidence": outcome.evidence,
                    "next_step": outcome.next_step,
                    "note": outcome.note,
                    "contract": outcome.contract,
                },
                "files_written": files,
            });
            Ok((value, op_id, id.to_string()))
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    ApiResponse::json(
        202,
        serde_json::json!({
            "agent": value,
            "contract": API_CONTRACT_VERSION,
            "project_id": id,
        }),
    )
}
