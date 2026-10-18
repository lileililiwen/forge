//! Operator views and immutable repository binding for hosted CI evidence.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route};
use super::gateway::{cors, error, guarded};
use serde_json::{json, Value};
use std::path::Path;

pub(in crate::api) fn route_hosted_ci(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "projects", id, "ci", "hosted"]) => {
            Some(Route::AdminHostedCiHistory {
                id: (*id).to_string(),
            })
        }
        ("PUT", ["v1", "admin", "projects", id, "ci", "repository"]) => {
            Some(Route::AdminHostedCiBind {
                id: (*id).to_string(),
            })
        }
        _ => None,
    }
}

pub(in crate::api) fn history(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let binding = match crate::hosted_ci::repository_binding(db_path, id) {
                Ok(value) => value,
                Err(error_value) => return hosted_ci_error(error_value),
            };
            let records = match crate::hosted_ci::observations(db_path, id, 20) {
                Ok(value) => value,
                Err(error_value) => return hosted_ci_error(error_value),
            };
            let current_revision = match crate::registry::Registry::open(db_path)
                .and_then(|registry| registry.inspect(id))
            {
                Ok(project) => crate::gate::capture_revision(Path::new(&project.path)),
                Err(error_value) => return ApiResponse::from_error(&error_value),
            };
            let rows: Vec<Value> = records.into_iter().map(|record| {
            let freshness = match current_revision.as_deref() {
                Some(revision) if revision == record.envelope.source_revision => "current",
                Some(_) => "stale",
                None => "unknown",
            };
            json!({"id":record.id,"envelope":record.envelope,"received_at":record.received_at,"freshness":freshness})
        }).collect();
            ApiResponse::json(
                200,
                json!({
                    "project_id":id,
                    "binding":binding.map(|(repository,repository_id)|json!({"repository":repository,"repository_id":repository_id})),
                    "observations":rows,
                    "contract":"forge-hosted-ci-evidence/1.0"
                }),
            )
        }),
    )
}

pub(in crate::api) fn bind(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            if !req
                .header("content-type")
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
            {
                return error(
                    415,
                    "admin-content-type-required",
                    "repository binding requires application/json",
                );
            }
            let body = req.json_body();
            if body.as_object().is_none_or(|object| object.len() != 1) {
                return error(
                    400,
                    "hosted-ci-invalid",
                    "repository binding accepts only repository_id",
                );
            }
            let Some(repository_id) = body.get("repository_id").and_then(Value::as_str) else {
                return error(
                    400,
                    "hosted-ci-invalid",
                    "repository_id must be a decimal string",
                );
            };
            match crate::hosted_ci::bind_repository_id(db_path, id, repository_id) {
                Ok(repository) => ApiResponse::json(
                    200,
                    json!({
                        "project_id":id,
                        "repository":repository,
                        "repository_id":repository_id,
                        "contract":"forge-hosted-ci-evidence/1.0"
                    }),
                ),
                Err(error_value) => hosted_ci_error(error_value),
            }
        }),
    )
}

fn hosted_ci_error(error_value: crate::hosted_ci::IngestError) -> ApiResponse {
    let (status, code, message) = match error_value {
        crate::hosted_ci::IngestError::Invalid(message) => (400, "hosted-ci-invalid", message),
        crate::hosted_ci::IngestError::Conflict(message) => (409, "hosted-ci-conflict", message),
        crate::hosted_ci::IngestError::UnknownProject => (
            404,
            "unknown-project",
            "registered project was not found".into(),
        ),
        crate::hosted_ci::IngestError::Unavailable(message) => {
            (503, "hosted-ci-unavailable", message)
        }
    };
    error(status, code, &message)
}
