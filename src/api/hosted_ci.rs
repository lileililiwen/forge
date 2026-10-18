//! Public GitHub Actions workload endpoint for hosted Gate observations.

use super::contract::API_CONTRACT_VERSION;
use super::model::{ApiRequest, ApiResponse};
use serde_json::json;
use std::path::Path;

pub(super) fn handle_ingest(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if request.body.len() > 256 * 1024 {
        return ApiResponse::json(
            413,
            json!({"error":{"code":"api-body-too-large","message":"hosted CI evidence exceeds the 256 KiB limit"},"contract":API_CONTRACT_VERSION}),
        );
    }
    let Some(token) = request.bearer_token.as_deref() else {
        return ApiResponse::json(
            401,
            json!({"error":{"code":"hosted-ci-unauthorized","message":"GitHub Actions ID token is required"},"contract":API_CONTRACT_VERSION}),
        );
    };
    let envelope = match serde_json::from_slice::<crate::ci_evidence::HostedCiEvidenceEnvelope>(
        &request.body,
    ) {
        Ok(value) => value,
        Err(_) => {
            return ApiResponse::json(
                400,
                json!({"error":{"code":"hosted-ci-invalid","message":"hosted CI evidence document is malformed"},"contract":API_CONTRACT_VERSION}),
            )
        }
    };
    if let Err(error) = envelope.validate() {
        return ApiResponse::json(
            400,
            json!({"error":{"code":"hosted-ci-invalid","message":error.to_string()},"contract":API_CONTRACT_VERSION}),
        );
    }
    let claims = match crate::hosted_ci::verify_github_actions_token_from_jwks(token) {
        Ok(value) => value,
        Err(_) => {
            return ApiResponse::json(
                401,
                json!({"error":{"code":"hosted-ci-unauthorized","message":"GitHub Actions identity could not be verified"},"contract":API_CONTRACT_VERSION}),
            )
        }
    };
    match crate::hosted_ci::ingest(db_path, envelope, &claims) {
        Ok(crate::hosted_ci::IngestOutcome::Inserted(record)) => ApiResponse::json(
            201,
            json!({
                "contract":"forge-hosted-ci-evidence/1.0","project_id":record.envelope.project_id,"id":record.id,"duplicate":false,"received_at":record.received_at
            }),
        ),
        Ok(crate::hosted_ci::IngestOutcome::Replayed(record)) => ApiResponse::json(
            200,
            json!({
                "contract":"forge-hosted-ci-evidence/1.0","project_id":record.envelope.project_id,"id":record.id,"duplicate":true,"received_at":record.received_at
            }),
        ),
        Err(error) => {
            let (status, code, message) = match error {
                crate::hosted_ci::IngestError::Invalid(message) => {
                    (403, "hosted-ci-binding-rejected", message)
                }
                crate::hosted_ci::IngestError::Conflict(message) => {
                    (409, "hosted-ci-conflict", message)
                }
                crate::hosted_ci::IngestError::UnknownProject => (
                    404,
                    "unknown-project",
                    "registered project was not found".to_string(),
                ),
                crate::hosted_ci::IngestError::Unavailable(message) => {
                    (503, "hosted-ci-unavailable", message)
                }
            };
            ApiResponse::json(
                status,
                json!({"error":{"code":code,"message":message},"contract":API_CONTRACT_VERSION}),
            )
        }
    }
}
