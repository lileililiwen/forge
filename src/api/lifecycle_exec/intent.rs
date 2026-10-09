use std::path::Path;

use serde_json::{json, Value};

use super::super::admin::{authoring_digest, cors, error, guarded, is_json, scrub_response};
use super::super::{ApiConfig, ApiRequest, ApiResponse};
use super::{
    confirm_note, digest_mismatch, latest_op_id, missing_digest, record_journal,
    resolve_project_dir, CONTRACT,
};

// ---- Intent ---------------------------------------------------------------

fn parse_intent_fields(body: &Value) -> Result<crate::planner::Intent, ApiResponse> {
    let action_raw = body
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("extend_project");
    let action = match action_raw {
        "create_project" => crate::planner::IntentAction::CreateProject,
        "extend_project" => crate::planner::IntentAction::ExtendProject,
        other => {
            return Err(ApiResponse::json(
                400,
                json!({
                    "error": { "code": "intent-invalid", "message": format!("unknown intent action '{other}'; accepted actions: create_project, extend_project") },
                    "effect": "none",
                    "contract": CONTRACT,
                }),
            ))
        }
    };
    let profile = body
        .get("profile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("rust-web")
        .to_string();
    let strings = |key: &str| -> Vec<String> {
        body.get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let required = strings("required");
    let forbidden = strings("forbidden");
    let constraints = body
        .get("constraints")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| {
                    let kv = v.as_str()?;
                    let (k, val) = kv.split_once('=')?;
                    if k.trim().is_empty() || val.trim().is_empty() {
                        return None;
                    }
                    Some(crate::planner::IntentConstraint {
                        key: k.trim().to_string(),
                        value: val.trim().to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(crate::planner::Intent {
        action,
        profile,
        required_capabilities: required,
        forbidden_capabilities: forbidden,
        constraints,
    })
}

fn intent_plan_view(intent: &crate::planner::Intent) -> Result<(Value, Value), ApiResponse> {
    let validated =
        crate::planner::validate_intent(intent).map_err(|e| ApiResponse::from_error(&e))?;
    let plan =
        crate::planner::resolve_plan(&validated, None).map_err(|e| ApiResponse::from_error(&e))?;
    let plan_value = serde_json::to_value(&plan).unwrap_or(Value::Null);
    let descriptor = json!({
        "action": "intent.apply",
        "plan": plan_value,
    });
    Ok((plan_value, descriptor))
}

pub fn intent_resolve(
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
                "intent resolve requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            if resolve_project_dir(db_path, id).is_err() {
                return resolve_project_dir(db_path, id).err().unwrap();
            }
            let body = req.json_body();
            let intent = match parse_intent_fields(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match intent_plan_view(&intent) {
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

pub fn intent_apply(
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
                "intent apply requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let (_pid, dir) = match resolve_project_dir(db_path, id) {
                Ok(v) => v,
                Err(r) => return scrub_response(r, &[]),
            };
            let body = req.json_body();
            let intent = match parse_intent_fields(&body) {
                Ok(v) => v,
                Err(r) => return r,
            };
            let (plan_value, descriptor) = match intent_plan_view(&intent) {
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
            let plan_id = plan_value
                .get("plan_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if plan_id.is_empty() {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": { "code": "intent-invalid", "message": "the resolved plan has no plan_id; nothing was written." },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            // Same in-process Core path as the CLI: persist the receipt,
            // then apply with explicit confirmation.
            let typed_plan: crate::planner::AssemblyPlan = match serde_json::from_value(
                plan_value.clone(),
            ) {
                Ok(p) => p,
                Err(_) => {
                    return ApiResponse::json(
                        409,
                        json!({
                            "error": { "code": "intent-invalid", "message": "the reviewed plan could not be reloaded; nothing was written." },
                            "effect": "none",
                            "contract": CONTRACT,
                        }),
                    )
                }
            };
            if let Err(e) = crate::planner::write_plan_receipt(&dir, &typed_plan) {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": { "code": "intent-invalid", "message": format!("the plan receipt could not be persisted: {e}; nothing was written.") },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            match crate::planner::apply_plan(&dir, &plan_id, true, Some(db_path)) {
                Ok(outcome) => {
                    let detail = format!(
                        "intent.apply {plan_id} applied={} stale={}",
                        outcome.applied_steps.len(),
                        outcome.stale
                    );
                    record_journal(db_path, "intent.apply", id, &detail);
                    let op_id = latest_op_id(db_path, "intent.apply", id);
                    ApiResponse::json(
                        200,
                        json!({
                            "accepted": true,
                            "project_id": id,
                            "plan_id": plan_id,
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
