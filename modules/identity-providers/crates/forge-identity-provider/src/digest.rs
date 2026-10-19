//! The plan digest.
//!
//! Forge Core shows a plan to the owner, asks for confirmation, and then applies the
//! exact plan the owner confirmed. The digest binds the confirmation to that plan.

use sha2::{Digest, Sha256};

use crate::response::ProposedChange;

/// The digest failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestError {
    /// The change list was empty, so the digest would not identify a plan.
    EmptyPlan,
}

impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPlan => write!(f, "a plan must name at least one change"),
        }
    }
}

impl std::error::Error for DigestError {}

/// Computes the digest of a plan.
///
/// The digest covers the project, the provider, the operation and every proposed
/// change, so a change to any of them produces a different digest.
pub fn plan_digest(
    project_id: &str,
    provider_id: &str,
    operation: &str,
    changes: &[ProposedChange],
) -> Result<String, DigestError> {
    if changes.is_empty() {
        return Err(DigestError::EmptyPlan);
    }

    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(provider_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(operation.as_bytes());
    hasher.update(b"\n");
    for change in changes {
        hasher.update(change.kind.as_bytes());
        hasher.update(b"\n");
        hasher.update(change.target.as_bytes());
        hasher.update(b"\n");
        hasher.update(change.summary.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
