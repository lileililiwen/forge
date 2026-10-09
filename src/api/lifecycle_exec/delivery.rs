use std::path::Path;

use serde_json::{json, Value};

use super::super::admin::{cors, error, guarded, is_json};
use super::super::{ApiConfig, ApiRequest, ApiResponse};
use super::{confirm_required, latest_op_id, record_journal, CONTRACT};

// ---- Delivery next-idea -----------------------------------------------------

fn credential_shaped(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("akia")
        || lower.contains("ghp_")
        || lower.contains("gho_")
        || lower.contains("begin private key")
        || lower.contains("xoxb-")
        || lower.contains("sk-ant-")
}

pub fn delivery_next_idea(
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
                "the next-idea transition requires application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            if crate::core::validate_project_id(id).is_err() {
                return ApiResponse::json(
                    400,
                    json!({
                        "error": { "code": "lifecycle-invalid", "message": "the project id is not a valid identifier." },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let body = req.json_body();
            if !body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return confirm_required("the next-idea transition");
            }
            let note = body
                .get("note")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            if note.len() > 2000 {
                return ApiResponse::json(
                    400,
                    json!({
                        "error": { "code": "lifecycle-invalid", "message": "the next-idea note is too long (max 2000 chars)." },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            if !note.is_empty() && credential_shaped(&note) {
                return ApiResponse::json(
                    400,
                    json!({
                        "error": { "code": "lifecycle-invalid", "message": "the note looks credential-shaped; refusing to journal it." },
                        "effect": "none",
                        "contract": CONTRACT,
                    }),
                );
            }
            let detail = if note.is_empty() {
                "delivery.next-idea: publish landed; next loop proposed".to_string()
            } else {
                format!("delivery.next-idea: {note}")
            };
            record_journal(db_path, "delivery.next-idea", id, &detail);
            let op_id = latest_op_id(db_path, "delivery.next-idea", id);
            ApiResponse::json(
                200,
                json!({
                    "accepted": true,
                    "project_id": id,
                    "operation_id": op_id,
                    "effect": "write",
                    "contract": CONTRACT,
                }),
            )
        }),
    )
}
