//! Versioned envelope for hosted project CI Gate observations.
//!
//! Validation establishes document shape and internal consistency only. It
//! does not authenticate a workflow, repository, timestamp, or reported result.

use chrono::DateTime;
use serde::{Deserialize, Serialize};

use crate::core::{ForgeError, validate_project_id};
use crate::gate::{GateAggregate, GateEvidence};
use crate::policy::redact_credentials;

pub const HOSTED_CI_EVIDENCE_CONTRACT: &str = "forge-hosted-ci-evidence/1.0";
const MAX_TEXT_CHARS: usize = 256;
const MAX_CHECKS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedCiEvidenceEnvelope {
    pub contract: String,
    pub project_id: String,
    /// Canonical GitHub repository slug (`owner/name`).
    pub repository: String,
    pub run: HostedCiRun,
    pub source_revision: String,
    pub gate: GateEvidence,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedCiRun {
    pub provider: String,
    pub workflow: String,
    pub run_id: String,
    pub run_attempt: u32,
    pub job: String,
    pub url: String,
}

impl HostedCiEvidenceEnvelope {
    /// Validate bounded fields and cross-field relationships. A valid envelope
    /// remains an untrusted producer claim until a transport authenticates it.
    pub fn validate(&self) -> Result<(), ForgeError> {
        let invalid = |reason: String| ForgeError::InvalidArgument { reason };
        if self.contract != HOSTED_CI_EVIDENCE_CONTRACT {
            return Err(invalid("unsupported hosted CI evidence contract".into()));
        }
        if self.gate.contract != crate::gate::GATE_CONTRACT_VERSION {
            return Err(invalid(
                "unsupported embedded Gate evidence contract".into(),
            ));
        }
        validate_project_id(&self.project_id).map_err(invalid)?;
        if self.gate.project_id != self.project_id {
            return Err(invalid(
                "project_id does not match embedded Gate evidence".into(),
            ));
        }
        if !valid_repository(&self.repository) {
            return Err(invalid(
                "repository must be a canonical owner/name slug".into(),
            ));
        }
        if self.run.provider != "github-actions" {
            return Err(invalid("run.provider must be github-actions".into()));
        }
        for (name, value) in [
            ("run.workflow", self.run.workflow.as_str()),
            ("run.job", self.run.job.as_str()),
        ] {
            if value.trim().is_empty() || value.chars().count() > MAX_TEXT_CHARS {
                return Err(invalid(format!(
                    "{name} is empty or exceeds the length limit"
                )));
            }
        }
        if self.run.run_attempt == 0
            || self.run.run_id.is_empty()
            || self.run.run_id.len() > 20
            || !self.run.run_id.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid(
                "run identity must contain a decimal run id and positive attempt".into(),
            ));
        }
        if !valid_sha(&self.source_revision)
            || self.gate.revision.as_deref() != Some(&self.source_revision)
        {
            return Err(invalid(
                "source_revision must be a full lowercase Git SHA matching Gate revision".into(),
            ));
        }
        if self.gate.dry_run || self.gate.aggregate == GateAggregate::Unknown {
            return Err(invalid(
                "Gate evidence must be a real run with a terminal aggregate".into(),
            ));
        }
        for (name, value) in [
            ("gate.runtime", self.gate.runtime.as_str()),
            (
                "gate.runtime_version",
                self.gate.runtime_version.as_deref().unwrap_or(""),
            ),
            (
                "gate.manifest_digest",
                self.gate.manifest_digest.as_deref().unwrap_or(""),
            ),
            (
                "gate.rule_pack_version",
                self.gate.rule_pack_version.as_deref().unwrap_or(""),
            ),
            ("gate.note", self.gate.note.as_deref().unwrap_or("")),
        ] {
            if value.chars().count() > MAX_TEXT_CHARS {
                return Err(invalid(format!("{name} exceeds the length limit")));
            }
            if redact_credentials(value) != value {
                return Err(invalid(format!("{name} contains credential-shaped text")));
            }
        }
        if self.gate.checks.len() > MAX_CHECKS {
            return Err(invalid(
                "Gate evidence exceeds the check count limit".into(),
            ));
        }
        for check in &self.gate.checks {
            if check.id.is_empty() || check.id.chars().count() > MAX_TEXT_CHARS {
                return Err(invalid(
                    "Gate check id is empty or exceeds the length limit".into(),
                ));
            }
            if check.note.as_ref().is_some_and(|note| {
                note.chars().count() > MAX_TEXT_CHARS || redact_credentials(note) != note.as_str()
            }) {
                return Err(invalid(
                    "Gate check note exceeds the limit or contains credential-shaped text".into(),
                ));
            }
        }
        validate_utc_timestamp("observed_at", &self.observed_at).map_err(invalid)?;
        validate_utc_timestamp("gate.observed_at", &self.gate.observed_at).map_err(invalid)?;
        validate_run_url(&self.run.url, &self.repository, &self.run.run_id).map_err(invalid)?;
        Ok(())
    }
}

fn valid_repository(repository: &str) -> bool {
    let mut parts = repository.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    [owner, name].into_iter().all(|part| {
        !part.is_empty()
            && part.len() <= MAX_TEXT_CHARS
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
            && !part.starts_with('.')
            && !part.ends_with('.')
    })
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn validate_utc_timestamp(name: &str, value: &str) -> Result<(), String> {
    if value.len() > MAX_TEXT_CHARS {
        return Err(format!("{name} exceeds the length limit"));
    }
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|_| format!("{name} must be RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!("{name} must use UTC"));
    }
    Ok(())
}

fn validate_run_url(url: &str, repository: &str, run_id: &str) -> Result<(), String> {
    if url.len() > MAX_TEXT_CHARS * 2 {
        return Err("run.url exceeds the length limit".into());
    }
    let parsed = reqwest::Url::parse(url).map_err(|_| "run.url is malformed".to_string())?;
    let expected = format!("/{repository}/actions/runs/{run_id}");
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != expected
    {
        return Err("run.url must be the matching HTTPS GitHub Actions run URL".into());
    }
    Ok(())
}
