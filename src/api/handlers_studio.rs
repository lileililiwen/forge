//! Studio project handlers (`handle_studio_*`).
//!
//! Typed HTTP handlers for the `/v1/projects/{id}/studio` surface:
//! spec get/save plus preview/refine. Bodies moved verbatim from
//! `src/api/mod.rs` by splitrs and sub-split by route family.
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::registry::Registry;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::contract::API_CONTRACT_VERSION;
use super::model::{ApiConfig, ApiRequest, ApiResponse};

pub(super) fn handle_studio_spec_save(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let spec_text = match body.get("spec").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-invalid-spec",
                        "message": "studio spec save requires a `spec` body field (YAML text)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let expected_revision = match body.get("expected_revision").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-revision-conflict",
                        "message": "studio spec save requires `expected_revision` (use `r0` for the first save)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let confirm = body.get("confirm").and_then(|v| v.as_str());
    if confirm != Some("yes") {
        return ApiResponse::json(
            400,
            serde_json::json!({
                "error": {
                    "code": "studio-invalid-spec",
                    "message": "studio spec save requires `confirm: yes`",
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let spec = match crate::studio::parse_spec_text(&spec_text) {
        Ok(spec) => spec,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::save_spec(&registry, id, &project_root, spec, &expected_revision) {
        Ok(session) => {
            let envelope = crate::studio::state::session_envelope(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "studio": envelope,
                    "contract": crate::studio::STUDIO_SESSION_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_studio_preview_get(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::load_session(&project_root) {
        Ok(Some(session)) => {
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Ok(None) => {
            let envelope = crate::studio::PreviewEnvelope::from_session(
                id,
                "r0",
                &crate::studio::state::SessionPreviewState::default(),
            );
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}

pub(super) fn handle_studio_preview_post(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let confirm = body.get("confirm").and_then(|v| v.as_str());
    if confirm != Some("yes") {
        return ApiResponse::from_error(&ForgeError::StudioInvalidSpec {
            reason: "studio preview requires `confirm: yes`".to_string(),
        });
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match action {
        "start" => {
            // The API server is long-lived, so it owns the live
            // session. Start the replacement first; only after it
            // reports ready do we drop any previous session (whose
            // drop kills only its own child).
            let runner = Box::new(crate::studio::ProcessRunner::react_web());
            let (session, live) =
                match crate::studio::start_preview(&registry, &project_root, runner) {
                    Ok(value) => value,
                    Err(err) => return ApiResponse::from_error(&err),
                };
            let previous = config.previews.lock().unwrap().insert(id.to_string(), live);
            drop(previous);
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        "stop" => {
            // Remove the live session first so a concurrent stop cannot
            // double-kill it, then let the Core record the idempotent
            // journal row and persist `stopped` (with or without a
            // held child).
            let live = config.previews.lock().unwrap().remove(id);
            match crate::studio::stop_preview(&registry, &project_root, live) {
                Ok(session) => {
                    let envelope = crate::studio::envelope_from_session(&session);
                    ApiResponse::json(
                        200,
                        serde_json::json!({
                            "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                            "contract": crate::studio::PREVIEW_CONTRACT,
                        }),
                    )
                }
                Err(err) => ApiResponse::from_error(&err),
            }
        }
        _ => ApiResponse::from_error(&ForgeError::StudioInvalidSpec {
            reason: "studio preview action must be `start` or `stop`".to_string(),
        }),
    }
}

pub(super) fn handle_studio_refine(
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    _now: DateTime<Utc>,
) -> ApiResponse {
    let body = request.json_body();
    let expected_revision = match body.get("expected_revision").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-revision-conflict",
                        "message": "studio refine requires `expected_revision` (use the current spec_revision or app_revision)",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let request_text = match body.get("request").and_then(|v| v.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return ApiResponse::json(
                400,
                serde_json::json!({
                    "error": {
                        "code": "studio-invalid-spec",
                        "message": "studio refine requires a `request` body field",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    };
    let selected_files: Vec<String> = body
        .get("selected_files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let record = match registry.inspect(id) {
        Ok(record) => record,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let project_root = PathBuf::from(&record.path);
    match crate::studio::record_refinement(
        &registry,
        &project_root,
        &expected_revision,
        &request_text,
        &selected_files,
    ) {
        Ok(session) => {
            let envelope = crate::studio::envelope_from_session(&session);
            ApiResponse::json(
                200,
                serde_json::json!({
                    "preview": serde_json::to_value(&envelope).unwrap_or(Value::Null),
                    "contract": crate::studio::PREVIEW_CONTRACT,
                }),
            )
        }
        Err(err) => ApiResponse::from_error(&err),
    }
}
