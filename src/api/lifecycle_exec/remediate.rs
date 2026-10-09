use std::path::Path;

use serde_json::{json, Value};

use super::super::admin::{authoring_digest, cors, error, guarded, is_json};
use super::super::{ApiConfig, ApiRequest, ApiResponse};
use super::{
    confirm_note, digest_mismatch, latest_op_id, missing_digest, record_journal,
    resolve_project_dir, CONTRACT,
};

// ---- Remediate -------------------------------------------------------------

fn remediate_fields(body: &Value) -> Result<(String, Option<String>), ApiResponse> {
    let finding = body
        .get("finding")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    if finding.is_empty() {
        return Err(ApiResponse::json(
            400,
            json!({
                "error": { "code": "remediation-invalid", "message": "enter a finding id to plan the remediation." },
                "effect": "none",
                "contract": CONTRACT,
            }),
        ));
    }
    let pack = body
        .get("pack")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok((finding, pack))
}

fn remediate_plan_view(
    dir: &Path,
    finding: &str,
    pack: Option<&str>,
) -> Result<(Value, Value), ApiResponse> {
    let plan = crate::remediation::build_plan(dir, finding, pack)
        .map_err(|e| ApiResponse::from_error(&e))?;
    let plan_value = serde_json::to_value(&plan).unwrap_or(Value::Null);
    let descriptor = json!({ "action": "remediate.apply", "plan": plan_value });
    Ok((plan_value, descriptor))
}

pub fn remediate_plan(
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
                "remediate plan requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let (_pid, dir) = match resolve_project_dir(db_path, id) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let body = req.json_body();
            let (finding, pack) = match remediate_fields(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match remediate_plan_view(&dir, &finding, pack.as_deref()) {
                Ok((plan, descriptor)) => ApiResponse::json(
                    200,
                    json!({
                        "preview": plan,
                        "plan": plan,
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

pub fn remediate_apply(
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
                "remediate apply requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let (_pid, dir) = match resolve_project_dir(db_path, id) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let body = req.json_body();
            let (finding, pack) = match remediate_fields(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let (plan_value, descriptor) =
                match remediate_plan_view(&dir, &finding, pack.as_deref()) {
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
                        "preview": plan_value,
                        "plan": plan_value,
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
                        "preview": plan_value,
                        "plan": plan_value,
                        "plan_digest": digest,
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let plan: crate::remediation::RemediationPlan =
                match serde_json::from_value(plan_value.clone()) {
                    Ok(p) => p,
                    Err(_) => {
                        return ApiResponse::from_error(
                            &crate::core::ForgeError::RemediationInvalid {
                                reason:
                                    "the reviewed plan could not be reloaded; nothing was written."
                                        .to_string(),
                            },
                        )
                    }
                };
            match crate::remediation::apply(&plan, true, db_path) {
                Ok(outcome) => {
                    let detail =
                        format!("remediate.apply {} status={}", plan.plan_id, outcome.status);
                    record_journal(db_path, "remediate.apply", id, &detail);
                    let op_id = latest_op_id(db_path, "remediate.apply", id);
                    ApiResponse::json(
                        200,
                        json!({
                            "accepted": true,
                            "project_id": id,
                            "plan_id": plan.plan_id,
                            "outcome": serde_json::to_value(&outcome).unwrap_or(Value::Null),
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
