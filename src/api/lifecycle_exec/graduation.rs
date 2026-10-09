use std::path::Path;

use serde_json::{json, Value};

use super::super::admin::{authoring_digest, cors, error, guarded, is_json};
use super::super::{ApiConfig, ApiRequest, ApiResponse};
use super::{
    confirm_note, digest_mismatch, latest_op_id, missing_digest, projects_root_response,
    record_journal, registry_unavailable, sha_hex, CONTRACT,
};
use crate::registry::Registry;

// ---- Graduation ------------------------------------------------------------

fn graduation_artifact(body: &Value) -> Result<String, ApiResponse> {
    let raw = body
        .get("artifact_json")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if raw.trim().is_empty() {
        return Err(ApiResponse::json(
            400,
            json!({
                "error": { "code": "graduation-invalid", "message": "paste the graduation artifact JSON into the artifact field." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    if raw.len() > crate::graduation::MAX_GRADUATION_BYTES {
        return Err(ApiResponse::json(
            400,
            json!({
                "error": { "code": "graduation-invalid", "message": "the artifact exceeds the 1MiB bound; nothing was read." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    Ok(raw)
}

fn graduation_profile(body: &Value) -> Result<String, ApiResponse> {
    let profile = body
        .get("profile")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    if profile.is_empty() {
        return Err(ApiResponse::json(
            400,
            json!({
                "error": { "code": "graduation-invalid", "message": "choose a profile for the import." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    Ok(profile)
}

fn graduation_id_override(body: &Value) -> Result<Option<String>, ApiResponse> {
    match body.get("id").and_then(Value::as_str).map(str::trim) {
        None | Some("") => Ok(None),
        Some(id) => {
            if crate::core::validate_project_id(id).is_err() {
                return Err(ApiResponse::json(
                    400,
                    json!({
                        "error": { "code": "graduation-conflict", "message": "the id override must use lowercase letters, numbers and dashes." },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                ));
            }
            Ok(Some(id.to_string()))
        }
    }
}

fn graduation_preview_view(
    raw: &str,
    profile: &str,
    id_override: Option<&str>,
    root: &Path,
) -> Result<(Value, Value), ApiResponse> {
    let record = crate::graduation::parse_artifact(raw).map_err(|r| graduation_refusal(&r))?;
    let import =
        crate::graduation::validate_graduation(&record).map_err(|r| graduation_refusal(&r))?;
    // Destination for proposal validation only: join server-side, never trust
    // a browser path. The id override selects the leaf; otherwise the title
    // kebab is derived by `build_proposal` against a scratch leaf.
    let scratch = match id_override {
        Some(id) => root.join(id),
        None => root.join("__graduation_preview__"),
    };
    let proposal = crate::graduation::build_proposal(&import, profile, &scratch, id_override)
        .map_err(|e| ApiResponse::from_error(&e))?;
    let preview = json!({
        "action": "graduation.import",
        "title": import.brief.title,
        "requirements": import.brief.requirements.len(),
        "evidence_count": import.experiment.evidence.len(),
        "source_contract": import.source.contract,
        "hypora_revision": import.source.hypora_revision,
        "proposed_id": proposal.id,
        "profile": proposal.profile,
    });
    let descriptor = json!({
        "action": "graduation.import",
        "artifact_sha256": sha_hex(raw.as_bytes()),
        "profile": profile,
        "id": id_override.unwrap_or(""),
        "proposed_id": proposal.id,
    });
    Ok((preview, descriptor))
}

fn graduation_refusal(r: &crate::graduation::GraduationRefusal) -> ApiResponse {
    ApiResponse::json(
        400,
        json!({
            "error": { "code": "graduation-invalid", "message": r.to_string() },
            "effect": "none",
            "contract": CONTRACT,
        }),
    )
}

pub fn graduation_preview(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "graduation preview requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let raw = match graduation_artifact(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let profile = match graduation_profile(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let id_override = match graduation_id_override(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let root = match projects_root_response() {
                Ok(r) => r,
                Err(r) => return r,
            };
            match graduation_preview_view(&raw, &profile, id_override.as_deref(), &root) {
                Ok((preview, descriptor)) => ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
                        "plan_digest": authoring_digest(&descriptor),
                        "confirmation": confirm_note(),
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                ),
                Err(r) => r,
            }
        }),
    )
}

pub fn graduation_import(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "graduation import requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let raw = match graduation_artifact(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let profile = match graduation_profile(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let id_override = match graduation_id_override(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let root = match projects_root_response() {
                Ok(r) => r,
                Err(r) => return r,
            };
            let (preview, descriptor) =
                match graduation_preview_view(&raw, &profile, id_override.as_deref(), &root) {
                    Ok(v) => v,
                    Err(r) => return r,
                };
            let digest = authoring_digest(&descriptor);
            if !body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
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
                        "preview": preview,
                        "plan_digest": digest,
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            // Same in-process Core path as the CLI.
            let record = match crate::graduation::parse_artifact(&raw) {
                Ok(r) => r,
                Err(r) => return graduation_refusal(&r),
            };
            let import = match crate::graduation::validate_graduation(&record) {
                Ok(i) => i,
                Err(r) => return graduation_refusal(&r),
            };
            let proposed_id = preview
                .get("proposed_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let dest = root.join(proposed_id);
            let proposal = match crate::graduation::build_proposal(
                &import,
                &profile,
                &dest,
                id_override.as_deref(),
            ) {
                Ok(p) => p,
                Err(e) => return ApiResponse::from_error(&e),
            };
            if dest.exists() {
                // `build_proposal` already refuses an occupied manifest, but
                // an occupied non-project directory is still a conflict here.
            }
            let mut registry = match Registry::open(db_path) {
                Ok(r) => r,
                Err(_) => return registry_unavailable(),
            };
            let now = chrono::Utc::now();
            match crate::graduation::adopt_graduation(
                &mut registry,
                &proposal,
                &import,
                "web-operator",
                now,
            ) {
                Ok(adoption) => {
                    let detail = format!(
                        "graduation.import {} profile={} receipt done",
                        proposal.id, proposal.profile
                    );
                    record_journal(db_path, "graduation.import", &proposal.id, &detail);
                    let op_id = latest_op_id(db_path, "graduation.import", &proposal.id);
                    ApiResponse::json(
                        200,
                        json!({
                            "accepted": true,
                            "project_id": proposal.id,
                            "receipt": adoption.receipt_path,
                            "operation_id": op_id,
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
