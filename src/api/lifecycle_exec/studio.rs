use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::super::admin::{authoring_digest, cors, error, guarded, is_json};
use super::super::{ApiConfig, ApiRequest, ApiResponse};
use super::{
    confirm_note, digest_mismatch, missing_digest, registry_unavailable, sha_hex, CONTRACT,
};
use crate::registry::Registry;

// ---- Studio (admin-gated wrappers) ------------------------------------------
// The bearer `/v1/projects/{id}/studio/*` routes need a project session;
// the workbench carries the admin session, so these wrappers expose the
// same in-process `save_spec` / `record_refinement` Core path (which journals
// `studio.spec.save` / `studio.refine` itself) behind the admin gate with
// the preview → confirm → digest discipline. The confirmed response echoes
// the `spec_revision` / `app_revision` pair so the card shows the bump.

pub fn studio_spec_save(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "studio spec save requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let spec_text = match body.get("spec").and_then(Value::as_str) {
                Some(v) if !v.trim().is_empty() => v.to_string(),
                _ => {
                    return ApiResponse::json(
                        400,
                        json!({
                            "error": { "code": "studio-invalid-spec", "message": "studio spec save requires a `spec` body field (YAML text)" },
                            "effect": "none",
                            "contract": CONTRACT,
                        }),
                    )
                }
            };
            let spec = match crate::studio::parse_spec_text(&spec_text) {
                Ok(s) => s,
                Err(e) => return ApiResponse::from_error(&e),
            };
            let descriptor = json!({
                "action": "studio.spec.save",
                "project_id": id,
                "spec_sha256": sha_hex(spec_text.as_bytes()),
            });
            let digest = authoring_digest(&descriptor);
            if !body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": descriptor,
                        "plan_digest": digest,
                        "confirmation": confirm_note(),
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let supplied = body
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            if supplied.is_empty() {
                return missing_digest();
            }
            if supplied != digest {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": digest_mismatch(),
                        "preview": descriptor,
                        "plan_digest": digest,
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let registry = match Registry::open(db_path) {
                Ok(r) => r,
                Err(_) => return registry_unavailable(),
            };
            let record = match registry.inspect(id) {
                Ok(r) => r,
                Err(e) => return ApiResponse::from_error(&e),
            };
            let root = PathBuf::from(&record.path);
            // First save uses `r0`; later saves must name the current
            // revision. The browser sends what it reviewed; a mismatch is
            // the typed revision-conflict refusal, nothing written.
            let expected = body
                .get("expected_revision")
                .and_then(Value::as_str)
                .unwrap_or("r0")
                .to_string();
            match crate::studio::save_spec(&registry, id, &root, spec, &expected) {
                Ok(session) => {
                    let envelope = crate::studio::state::session_envelope(&session);
                    ApiResponse::json(
                        200,
                        json!({
                            "accepted": true,
                            "project_id": id,
                            "preview": envelope,
                            "spec_revision": envelope.get("spec_revision"),
                            "app_revision": envelope.get("app_revision"),
                            "effect": "write",
                            "contract": CONTRACT,
                        }),
                    )
                }
                Err(e) => ApiResponse::from_error(&e),
            }
        }),
    )
}

pub fn studio_refine(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "studio refine requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let request_text = match body.get("request").and_then(Value::as_str) {
                Some(v) if !v.trim().is_empty() => v.to_string(),
                _ => {
                    return ApiResponse::json(
                        400,
                        json!({
                            "error": { "code": "studio-invalid-spec", "message": "studio refine requires a `request` body field" },
                            "effect": "none",
                            "contract": CONTRACT,
                        }),
                    )
                }
            };
            let expected = match body.get("expected_revision").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => {
                    return ApiResponse::json(
                        400,
                        json!({
                            "error": { "code": "studio-revision-conflict", "message": "studio refine requires `expected_revision` (use the current spec_revision or app_revision)" },
                            "effect": "none",
                            "contract": CONTRACT,
                        }),
                    )
                }
            };
            let selected: Vec<String> = body
                .get("selected_files")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let descriptor = json!({
                "action": "studio.refine",
                "project_id": id,
                "expected_revision": expected,
                "request_sha256": sha_hex(request_text.as_bytes()),
                "selected_files": selected,
            });
            let digest = authoring_digest(&descriptor);
            if !body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": descriptor,
                        "plan_digest": digest,
                        "confirmation": confirm_note(),
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let supplied = body
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            if supplied.is_empty() {
                return missing_digest();
            }
            if supplied != digest {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": digest_mismatch(),
                        "preview": descriptor,
                        "plan_digest": digest,
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let registry = match Registry::open(db_path) {
                Ok(r) => r,
                Err(_) => return registry_unavailable(),
            };
            let record = match registry.inspect(id) {
                Ok(r) => r,
                Err(e) => return ApiResponse::from_error(&e),
            };
            let root = PathBuf::from(&record.path);
            match crate::studio::record_refinement(
                &registry,
                &root,
                &expected,
                &request_text,
                &selected,
            ) {
                Ok(session) => {
                    let envelope = crate::studio::envelope_from_session(&session);
                    let value = serde_json::to_value(&envelope).unwrap_or(Value::Null);
                    ApiResponse::json(
                        200,
                        json!({
                            "accepted": true,
                            "project_id": id,
                            "preview": value,
                            "spec_revision": session.spec_revision,
                            "app_revision": session.app_revision,
                            "effect": "write",
                            "contract": CONTRACT,
                        }),
                    )
                }
                Err(e) => ApiResponse::from_error(&e),
            }
        }),
    )
}
