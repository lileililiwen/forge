//! Semantic review: decide.

use crate::core::ForgeError;
use crate::semantic::proposal::{Proposal, ProposalId, ProposalState, SEMANTIC_DIR};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use super::model::{DecideOutcome, ProposalListEntry};

use super::suggest::{project_id_for, proposal_dir, write_manifest};

/// Move a `Suggested` proposal to `Approved`. The operation
/// refuses a stale proposal (already `Superseded`, `Conflicted`,
/// or `Approved`) and never applies the suggested value to any
/// surface; the remediation and adapter packages own the actual
/// write.
pub fn approve(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
    confirm: bool,
) -> Result<DecideOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::SemanticInvalid {
            reason: "approve requires explicit --confirm; no proposal was changed".to_string(),
        });
    }
    decide(project_path, id, ProposalState::Approved, now)
}

/// Move a `Suggested` proposal to `Rejected`. The operation
/// refuses a stale proposal and never touches the project or any
/// provider; the current value stays in force.
pub fn reject(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
    confirm: bool,
) -> Result<DecideOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::SemanticInvalid {
            reason: "reject requires explicit --confirm; no proposal was changed".to_string(),
        });
    }
    decide(project_path, id, ProposalState::Rejected, now)
}

/// Mark a proposal as superseded. The function is exposed for
/// tests; the production `suggest` path drives the same
/// transition implicitly. Calling it on a proposal that is not
/// currently `Suggested` is refused.
pub fn supersede(
    project_path: &Path,
    id: &ProposalId,
    now: DateTime<Utc>,
) -> Result<DecideOutcome, ForgeError> {
    decide(project_path, id, ProposalState::Superseded, now)
}

/// Read a single proposal. Returns `Ok(None)` when the manifest
/// does not exist; returns `Ok(Some(_))` for a valid manifest;
/// returns `SemanticInvalid` for a malformed manifest.
pub fn read(project_path: &Path, id: &ProposalId) -> Result<Option<Proposal>, ForgeError> {
    let path = proposal_dir(project_path, id).join("manifest.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!("cannot read proposal manifest {}: {err}", path.display()),
    })?;
    let proposal: Proposal =
        serde_json::from_str(&text).map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("malformed proposal manifest {}: {err}", path.display()),
        })?;
    Ok(Some(proposal))
}

/// List all proposals under `.forge/semantic/<project_id>/`.
/// Malformed entries are skipped so a partial write never breaks
/// the listing.
pub fn list(project_path: &Path) -> Result<Vec<ProposalListEntry>, ForgeError> {
    let project_id = project_id_for(project_path)?;
    let base = project_path.join(SEMANTIC_DIR).join(&project_id);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&base).map_err(|err| ForgeError::SemanticInvalid {
        reason: format!("cannot read {}: {err}", base.display()),
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let text = match fs::read_to_string(&manifest_path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let proposal: Proposal = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(_) => continue,
        };
        out.push(ProposalListEntry {
            id: proposal.id.clone(),
            project_id: proposal.project_id.clone(),
            kind: proposal.kind,
            state: proposal.state,
            confidence: proposal.confidence,
            provider: proposal.provider,
            evidence_count: proposal.evidence.len(),
            suggested_at: proposal.suggested_at,
            decided_at: proposal.decided_at,
            dir_name: proposal.id.dir_name(),
        });
    }
    out.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    Ok(out)
}

fn decide(
    project_path: &Path,
    id: &ProposalId,
    target: ProposalState,
    now: DateTime<Utc>,
) -> Result<DecideOutcome, ForgeError> {
    let mut current = read(project_path, id)?.ok_or_else(|| ForgeError::SemanticInvalid {
        reason: format!("proposal `{}` was not found", id.dir_name()),
    })?;
    if !current.state.operator_transitionable() {
        return Err(ForgeError::SemanticConflict {
            reason: format!(
                "proposal `{}` is in state `{}`; only `suggested` proposals can be approved or rejected",
                id.dir_name(),
                current.state
            ),
        });
    }
    if matches!(target, ProposalState::Approved | ProposalState::Rejected) {
        // No behaviour difference from the user-facing API; the
        // check is explicit so the function cannot be misused as
        // a "force" path.
    }
    current.state = target;
    current.decided_at = Some(now);
    let files = write_manifest(project_path, &current)?;
    Ok(DecideOutcome {
        id: id.clone(),
        state: target,
        files_written: files,
        note: format!(
            "proposal `{}` moved to state `{}`; no project file or provider was changed",
            id.dir_name(),
            target
        ),
    })
}
