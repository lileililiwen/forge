//! Web surface for the gate evidence envelope (`forge-web-gate-controls`).
//!
//! Two session-gated admin routes that wrap the existing `forge::gate`
//! Core path the CLI uses, so a sibling operator can read the current
//! cached gate status and ask Forge to re-run the gate and journal a
//! fresh evidence record from the browser. The browser is a thin caller:
//! the handler delegates to `forge::gate::run_gate` for the run path and
//! `forge::gate::load_latest_evidence` + `evidence_freshness` for the
//! status path, and never classifies a state the runtime did not report.
//!
//! A parseable document wins over the exit code (the sibling exits
//! non-zero exactly when blocked); a blocked document is a recorded
//! verdict, not an adapter failure. Unparseable output, a timeout, a
//! spawn failure, a rehearsal document on a real run, or a missing
//! runtime is `Unavailable` / `409 gate-rehearsal-not-evidence` and
//! leaves any previously persisted evidence byte-identical. The handler
//! never sets `GateAggregate::Passed` unless the runtime produced a
//! parseable `PASS` document with a successful exit; the same rule
//! `classify_aggregate` already enforces.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text};

/// Match the two gate routes before the main table runs. The literal
/// `gate`/`status`/`run` segments never collide with the existing
/// `health`/`refresh`/`classify`/`agents`/`identity` arms above, so
/// trying these shapes first shadows no existing route. Kept beside the
/// handlers (not in `router.rs`) so the route table stays under the
/// source-file-size cap.
pub(in crate::api) fn route_gate(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "projects", id, "gate", "status"]) => {
            Some(Route::AdminProjectGateStatus {
                id: (*id).to_string(),
            })
        }
        ("POST", ["v1", "admin", "projects", id, "gate", "run"]) => {
            Some(Route::AdminProjectGateRun {
                id: (*id).to_string(),
            })
        }
        _ => None,
    }
}

/// Render a `ForgeError` with the shared status mapping, scrubbing the
/// project directory from the message. The typed code and status are
/// preserved so a boundary scenario is reported honestly.
fn typed_gate_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let status = super::super::err_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Resolve the project directory server-side. The browser sends only a
/// validated id; the server validates again (`deploy_id_gate`) and opens
/// the registered root — never a browser-supplied path.
fn resolve_project(db_path: &Path, id: &str) -> Result<std::path::PathBuf, ApiResponse> {
    deploy_id_gate(db_path, id)
}

/// Project the parsed `GateEvidence` onto the wire envelope, overlaid
/// with the current freshness. The same projection runs for both the
/// read and the run paths so the browser, the CLI, and the doctor's
/// `gate-evidence` finding all report the same aggregate, the same
/// per-check states, the same short revision, and the same freshness
/// for the same project on the same revision.
fn evidence_envelope(
    project_id: &str,
    evidence: &crate::gate::GateEvidence,
    freshness: crate::gate::GateFreshness,
) -> Value {
    let mut value = match serde_json::to_value(evidence) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "freshness".to_string(),
            serde_json::json!(freshness.label()),
        );
    }
    json!({
        "contract": crate::gate::GATE_CONTRACT_VERSION,
        "project_id": project_id,
        "gate": value,
    })
}

/// `GET /v1/admin/projects/{id}/gate/status` — read-only. Reuses the same
/// Core functions the CLI uses for `forge gate status` (`load_latest_evidence`
/// + `evidence_freshness` + `evidence_summary`). The handler renders the
/// parsed `GateEvidence` envelope; the cached record is read-only and no
/// write path runs. The `GateEvidence` record is the same JSON the
/// `cmd_gate_status` envelope flattens, so a browser read and a CLI
/// read on the same project on the same revision produce the same wire.
pub(in crate::api) fn gate_status(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match resolve_project(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::gate::load_latest_evidence(&project_dir) {
                Ok(Some(evidence)) => {
                    let current = crate::gate::capture_revision(&project_dir);
                    let freshness = crate::gate::evidence_freshness(&evidence, current.as_deref());
                    let summary = crate::gate::evidence_summary(&evidence);
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "contract": crate::gate::GATE_CONTRACT_VERSION,
                                "project_id": id,
                                "summary": summary,
                                "envelope": evidence_envelope(id, &evidence, freshness),
                                "contract_version": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                Ok(None) => ApiResponse::json(
                    200,
                    json!({
                        "contract": crate::gate::GATE_CONTRACT_VERSION,
                        "project_id": id,
                        "summary": "unverified",
                        "envelope": {
                            "contract": crate::gate::GATE_CONTRACT_VERSION,
                            "project_id": id,
                            "gate": null,
                            "freshness": crate::gate::GateFreshness::Absent.label(),
                            "detail": "the gate has never run; unverified is not a pass",
                        },
                        "contract_version": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_gate_error(&project_dir, &err),
            }
        }),
    )
}

/// `POST /v1/admin/projects/{id}/gate/run` — explicit on-demand re-run.
/// Reuses the same `forge::gate::run_gate` path the CLI uses for
/// `forge gate {id}` (`src/cli/functions_13.rs:573`), the same
/// `forge::gate::save_evidence` atomic tmp+rename the CLI uses, and the
/// same `registry.record_operation("gate", id, verdict, summary)` call
/// the CLI uses. The aggregate and per-check states are taken from
/// what the runtime actually said; the handler never fabricates
/// `passed` from a missing runtime answer.
pub(in crate::api) fn gate_run(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    if !super::gateway::is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "gate run requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match resolve_project(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            let registry = match crate::registry::Registry::open(db_path) {
                Ok(registry) => registry,
                Err(err) => {
                    return ApiResponse::json(
                        503,
                        json!({
                            "error": {
                                "code": "admin-registry-unavailable",
                                "message": err.to_string(),
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    );
                }
            };
            let config = crate::gate::GateConfig::from_env();
            let outcome = crate::gate::run_gate(&project_dir, id, &config, false);
            match outcome {
                crate::gate::GateOutcome::Evidence(evidence) => {
                    // The CLI's contract: a real-run evidence record is the
                    // only outcome that may be persisted and journaled.
                    // Persist atomically (tmp+rename); a failure is reported
                    // with the typed Forge error and the prior evidence
                    // stays byte-identical.
                    let persisted = match crate::gate::save_evidence(&project_dir, &evidence) {
                        Ok(relative) => Some(relative.display().to_string()),
                        Err(err) => {
                            return typed_gate_error(&project_dir, &err);
                        }
                    };
                    let verdict = crate::gate::journal_verdict(
                        &crate::gate::GateOutcome::Evidence(evidence.clone()),
                    );
                    let summary = crate::gate::evidence_summary(&evidence);
                    let _ = registry.record_operation("gate", id, verdict, &summary);
                    let current = crate::gate::capture_revision(&project_dir);
                    let freshness = crate::gate::evidence_freshness(&evidence, current.as_deref());
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "contract": crate::gate::GATE_CONTRACT_VERSION,
                                "project_id": id,
                                "summary": summary,
                                "envelope": evidence_envelope(id, &evidence, freshness),
                                "persisted": persisted,
                                "journaled": true,
                                "contract_version": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                crate::gate::GateOutcome::Rehearsal { runtime, .. } => {
                    // A rehearsal answers nothing; a rehearsal answering a
                    // real run is a contradiction, reported as a typed
                    // 409 so the browser can show the refusal and prior
                    // evidence stays untouched. Never evidence, never a
                    // journal row, never a pass.
                    ApiResponse::json(
                        409,
                        json!({
                            "error": {
                                "code": "gate-rehearsal-not-evidence",
                                "message": format!(
                                    "gate runtime `{runtime}` answered a plan rehearsal; a rehearsal executes nothing and is never evidence, never a pass. The cached evidence (if any) is untouched."
                                ),
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                crate::gate::GateOutcome::Unavailable { reason } => {
                    // Runtime unresolvable, timed out, or unparseable: a
                    // typed 502 with the bounded reason. Prior evidence
                    // stays byte-identical (`src/gate/mod.rs:33-36`).
                    let bound = crate::gate::bound_note(&reason);
                    ApiResponse::json(
                        502,
                        json!({
                            "error": {
                                "code": "gate-runtime-unavailable",
                                "message": bound,
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                crate::gate::GateOutcome::PlanPreview { runtime, .. } => {
                    // A dry-run answer is never a verdict; the browser
                    // never asks for a dry-run, so reaching this arm is a
                    // sibling-version regression. Reported the same way as
                    // `Unavailable`: typed refusal, prior evidence
                    // untouched, no journal row, never a pass.
                    ApiResponse::json(
                        502,
                        json!({
                            "error": {
                                "code": "gate-runtime-unavailable",
                                "message": format!(
                                    "gate runtime `{runtime}` answered a dry-run preview instead of a verdict; the cached evidence is untouched."
                                ),
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
            }
        }),
    )
}
