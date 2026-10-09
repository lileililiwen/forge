//! Portfolio write validation.

use crate::core::ForgeError;
use crate::portfolio;
use chrono::{DateTime, Utc};

use super::writes::SnapshotWrite;

/// Validate one evidence write end to end without touching the
/// database. Shared by the CLI, the JSON API and the portal so
/// every caller applies the same bounds.
pub fn validate_snapshot(write: &SnapshotWrite) -> Result<SnapshotWrite, ForgeError> {
    let source_system = portfolio::validate_source_system(&write.source_system)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    let source_revision = portfolio::validate_source_revision(&write.source_revision)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    let observed_at = parse_timestamp("observed_at", &write.observed_at)?;
    let stale_after = match &write.stale_after {
        Some(value) => Some(parse_timestamp("stale_after", value)?),
        None => None,
    };
    if let Some(bound) = &stale_after {
        if bound < &observed_at {
            return Err(ForgeError::PortfolioInvalid {
                reason:
                    "stale_after precedes observed_at; a snapshot cannot expire before it was seen"
                        .to_string(),
            });
        }
    }
    let evidence_json = portfolio::prepare_evidence(&write.evidence_json)
        .map_err(|reason| ForgeError::PortfolioInvalid { reason })?;
    Ok(SnapshotWrite {
        source_system,
        source_revision,
        observed_at,
        status: write.status,
        stale_after,
        evidence_json,
    })
}

/// Parse one RFC 3339 timestamp into the canonical string form.
fn parse_timestamp(field: &str, raw: &str) -> Result<String, ForgeError> {
    let trimmed = raw.trim();
    DateTime::parse_from_rfc3339(trimmed)
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| ForgeError::PortfolioInvalid {
            reason: format!("{field} must be an RFC 3339 timestamp, got `{trimmed}`"),
        })
}

pub(super) fn invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInvalid { reason }
}
