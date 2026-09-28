//! Approval and publication audit shapes.
//!
//! These rows are the record of *what was public*: which manifest
//! revision, which hash, which actor, which outcome. They are written
//! once and never rewritten, so the trail can answer a question about
//! any past publication.

use serde::Serialize;

use crate::portfolio::share::validation::validate_public_text;

// --- approval and publication records -----------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ShareApproval {
    pub revision: i64,
    pub manifest_sha256: String,
    pub manifest_json: String,
    pub project_count: i64,
    pub actor: String,
    pub approved_at: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicationAttempt {
    pub publication_id: i64,
    pub operation_key: String,
    pub manifest_revision: i64,
    pub manifest_sha256: String,
    pub target: String,
    pub status: String,
    pub published_revision: Option<String>,
    pub error_code: Option<String>,
    pub actor: String,
    pub attempted_at: String,
    pub finished_at: Option<String>,
    pub document_generated_at: String,
}

/// Validate an operation key. A retry must be able to name the same
/// operation, so the shape is a short printable token with no
/// whitespace.
pub fn validate_operation_key(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("operation key is required".to_string());
    }
    if trimmed.len() > 128 {
        return Err("operation key is longer than 128 characters".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(
            "operation key must use only letters, digits, dashes, underscores and dots".to_string(),
        );
    }
    Ok(trimmed.to_string())
}

/// Validate an actor identity recorded in the audit trail.
pub fn validate_actor(raw: &str) -> Result<String, String> {
    validate_public_text("actor", raw, 128, false)
}
