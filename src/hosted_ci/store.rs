//! Registry binding and append-only hosted CI observation storage.

use super::{validate_claims_for_envelope, GitHubActionsClaims};
use crate::ci_evidence::HostedCiEvidenceEnvelope;
use crate::registry::Registry;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedCiObservation {
    pub id: String,
    pub envelope: HostedCiEvidenceEnvelope,
    pub received_at: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestOutcome {
    Inserted(HostedCiObservation),
    Replayed(HostedCiObservation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    Invalid(String),
    Conflict(String),
    UnknownProject,
    Unavailable(String),
}

/// Normalize the registered remote to GitHub's canonical owner/name slug.
pub fn github_slug(remote: &str) -> Option<String> {
    let mut value = remote.trim();
    if let Some(stripped) = value.strip_prefix("git@github.com:") {
        value = stripped;
    } else if let Some(stripped) = value.strip_prefix("ssh://git@github.com/") {
        value = stripped;
    } else if let Some(stripped) = value.strip_prefix("https://github.com/") {
        value = stripped;
    } else {
        return None;
    }
    let slug = value
        .strip_suffix(".git")
        .unwrap_or(value)
        .trim_end_matches('/');
    let mut parts = slug.split('/');
    let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if [owner, repo].into_iter().any(|part| {
        part.is_empty()
            || part.len() > 100
            || !part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || part.starts_with('.')
            || part.ends_with('.')
    }) {
        return None;
    }
    Some(format!(
        "{}/{}",
        owner.to_ascii_lowercase(),
        repo.to_ascii_lowercase()
    ))
}

fn registry_error(error: impl std::fmt::Display) -> IngestError {
    IngestError::Unavailable(format!("hosted CI registry operation failed: {error}"))
}

pub fn bind_repository_id(
    db_path: &Path,
    project_id: &str,
    repository_id: &str,
) -> Result<String, IngestError> {
    if repository_id.is_empty()
        || repository_id.len() > 20
        || repository_id.starts_with('0')
        || !repository_id.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(IngestError::Invalid(
            "repository ID must be a positive decimal integer".into(),
        ));
    }
    let mut registry = Registry::open(db_path).map_err(registry_error)?;
    let project = registry.inspect(project_id).map_err(registry_error)?;
    let slug = project
        .git_remote
        .as_deref()
        .and_then(github_slug)
        .ok_or_else(|| {
            IngestError::Invalid(
                "registered project remote must resolve to one GitHub owner/name slug".into(),
            )
        })?;
    let tx = registry
        .registry_conn_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(registry_error)?;
    let current: Option<String> = tx
        .query_row(
            "SELECT repository_id FROM hosted_ci_project_bindings WHERE project_id = ?1",
            params![project.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(registry_error)?;
    if let Some(current) = current {
        if current == repository_id {
            return Ok(slug);
        }
        return Err(IngestError::Conflict(
            "project is already bound to a different GitHub repository ID".into(),
        ));
    }
    if tx
        .query_row(
            "SELECT project_id FROM hosted_ci_project_bindings WHERE repository_id = ?1",
            params![repository_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(registry_error)?
        .is_some()
    {
        return Err(IngestError::Conflict(
            "GitHub repository ID is already bound to another Forge project".into(),
        ));
    }
    tx.execute(
        "INSERT INTO hosted_ci_project_bindings (project_id, repository_id, repository)
         VALUES (?1, ?2, ?3)",
        params![project.id, repository_id, slug],
    )
    .map_err(registry_error)?;
    tx.commit().map_err(registry_error)?;
    Ok(slug)
}

pub fn repository_binding(
    db_path: &Path,
    project_id: &str,
) -> Result<Option<(String, String)>, IngestError> {
    let registry = Registry::open(db_path).map_err(registry_error)?;
    let project = registry.inspect(project_id).map_err(registry_error)?;
    registry
        .registry_conn()
        .query_row(
            "SELECT repository, repository_id FROM hosted_ci_project_bindings WHERE project_id = ?1",
            params![project.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(registry_error)
}

pub fn ingest(
    db_path: &Path,
    envelope: HostedCiEvidenceEnvelope,
    claims: &GitHubActionsClaims,
) -> Result<IngestOutcome, IngestError> {
    envelope
        .validate()
        .map_err(|error| IngestError::Invalid(error.to_string()))?;
    let mut registry = Registry::open(db_path).map_err(registry_error)?;
    let mut matching_projects = registry
        .list()
        .map_err(registry_error)?
        .into_iter()
        .filter(|project| {
            project
                .git_remote
                .as_deref()
                .and_then(github_slug)
                .as_deref()
                == Some(envelope.repository.to_ascii_lowercase().as_str())
        })
        .collect::<Vec<_>>();
    if matching_projects.is_empty() {
        return Err(IngestError::UnknownProject);
    }
    if matching_projects.len() != 1 {
        return Err(IngestError::Invalid(
            "repository slug maps to multiple registered Forge projects".into(),
        ));
    }
    let project = matching_projects.remove(0);
    if project.id != envelope.project_id {
        return Err(IngestError::Invalid(
            "envelope project does not match the registered repository".into(),
        ));
    }
    let (bound_slug, repository_id) =
        repository_binding(db_path, &project.id)?.ok_or_else(|| {
            IngestError::Invalid("project has no trusted GitHub repository binding".into())
        })?;
    if bound_slug != envelope.repository.to_ascii_lowercase() {
        return Err(IngestError::Invalid(
            "registered repository slug does not match envelope".into(),
        ));
    }
    validate_claims_for_envelope(claims, &repository_id, &envelope)
        .map_err(IngestError::Invalid)?;
    let body = serde_json::to_vec(&envelope).map_err(registry_error)?;
    if body.len() > 256 * 1024 {
        return Err(IngestError::Invalid(
            "hosted CI evidence exceeds the size limit".into(),
        ));
    }
    let digest = format!("{:x}", Sha256::digest(&body));
    let id_material = format!(
        "{}:{}:{}",
        project.id, envelope.run.run_id, envelope.run.run_attempt
    );
    let id = format!(
        "hci-{}",
        &format!("{:x}", Sha256::digest(id_material.as_bytes()))[..24]
    );
    let tx = registry
        .registry_conn_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(registry_error)?;
    let existing: Option<(String, String, String)> = tx
        .query_row(
            "SELECT id, digest, envelope_json FROM hosted_ci_evidence
             WHERE project_id = ?1 AND run_id = ?2 AND run_attempt = ?3",
            params![project.id, envelope.run.run_id, envelope.run.run_attempt],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(registry_error)?;
    if let Some((existing_id, existing_digest, existing_json)) = existing {
        if existing_digest != digest {
            return Err(IngestError::Conflict(
                "hosted CI run identity already contains different evidence".into(),
            ));
        }
        let existing_envelope = serde_json::from_str(&existing_json).map_err(registry_error)?;
        let received_at = tx
            .query_row(
                "SELECT received_at FROM hosted_ci_evidence WHERE id = ?1",
                params![existing_id],
                |row| row.get(0),
            )
            .map_err(registry_error)?;
        return Ok(IngestOutcome::Replayed(HostedCiObservation {
            id: existing_id,
            envelope: existing_envelope,
            received_at,
            digest: existing_digest,
        }));
    }
    let received_at = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO hosted_ci_evidence
         (id, project_id, repository, repository_id, run_id, run_attempt, job, workflow,
          source_revision, aggregate, observed_at, received_at, digest, envelope_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id,
            project.id,
            envelope.repository,
            repository_id,
            envelope.run.run_id,
            envelope.run.run_attempt,
            envelope.run.job,
            envelope.run.workflow,
            envelope.source_revision,
            serde_json::to_value(envelope.gate.aggregate)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".into()),
            envelope.observed_at,
            received_at,
            digest,
            String::from_utf8_lossy(&body).as_ref(),
        ],
    )
    .map_err(registry_error)?;
    tx.commit().map_err(registry_error)?;
    Ok(IngestOutcome::Inserted(HostedCiObservation {
        id,
        envelope,
        received_at,
        digest,
    }))
}

pub fn observations(
    db_path: &Path,
    project_id: &str,
    limit: usize,
) -> Result<Vec<HostedCiObservation>, IngestError> {
    let registry = Registry::open(db_path).map_err(registry_error)?;
    registry.inspect(project_id).map_err(registry_error)?;
    let mut statement = registry
        .registry_conn()
        .prepare(
            "SELECT id, envelope_json, received_at, digest FROM hosted_ci_evidence
             WHERE project_id = ?1 ORDER BY received_at DESC, id DESC LIMIT ?2",
        )
        .map_err(registry_error)?;
    let rows = statement
        .query_map(params![project_id, limit.clamp(1, 100) as i64], |row| {
            let json: String = row.get(1)?;
            Ok((
                row.get::<_, String>(0)?,
                json,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(registry_error)?;
    let mut result = Vec::new();
    for row in rows {
        let (id, json, received_at, digest) = row.map_err(registry_error)?;
        let envelope = serde_json::from_str(&json).map_err(registry_error)?;
        result.push(HostedCiObservation {
            id,
            envelope,
            received_at,
            digest,
        });
    }
    Ok(result)
}
