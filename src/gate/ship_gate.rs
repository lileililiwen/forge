//! The shared delivery rule (`single-delivery-lane`).
//!
//! One owner decides whether a project may deliver a revision. The
//! rule reads the Gate evidence the project already persisted and
//! compares it with the revision that is about to be delivered. Every
//! delivery lane calls it, so no lane can put an artifact on a target
//! the Gate has not verified.
//!
//! The rule runs nothing and writes nothing: it only reads the
//! evidence file. A missing or unreadable record counts as absent, so
//! a corrupt file can never read as a passing verification.

use std::path::Path;

use serde::Serialize;

use super::contract::is_supported_gate_contract;
use super::engine::{evidence_freshness, load_latest_evidence};
use super::model::{GateAggregate, GateFreshness, GateProfile};

/// The verdict of the delivery rule for one project revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShipGateStatus {
    /// True when the project may deliver this revision.
    pub satisfied: bool,
    /// The reason the project may not deliver. Empty when it may.
    pub reason: String,
}

impl ShipGateStatus {
    /// An allowed verdict: the project may deliver this revision.
    fn allow() -> Self {
        ShipGateStatus {
            satisfied: true,
            reason: String::new(),
        }
    }

    /// A refusal: the project may not deliver, and `reason` says why.
    fn refuse(reason: String) -> Self {
        ShipGateStatus {
            satisfied: false,
            reason,
        }
    }
}

/// Decide whether a project may deliver `revision`.
///
/// The conditions are exactly the delivery lane's conditions, moved:
/// the evidence must use a supported contract, come from the full
/// profile, not be a rehearsal (dry-run) record, belong to `project_id`,
/// be fresh for `revision`, and have a passed aggregate.
///
/// The verdict is a value, not an error: the deployment plan reports
/// it and the delivery lanes refuse on it, and each lane keeps its own
/// error vocabulary.
pub fn evaluate_ship_gate(project_dir: &Path, project_id: &str, revision: &str) -> ShipGateStatus {
    let evidence = match load_latest_evidence(project_dir) {
        Ok(Some(evidence)) => evidence,
        // A missing or unreadable record is an absence: the owner is
        // told to run the Gate, never handed a verdict it cannot trust.
        Ok(None) | Err(_) => return absent(project_id),
    };
    let freshness = evidence_freshness(&evidence, Some(revision));
    if evidence.project_id != project_id
        || !is_supported_gate_contract(&evidence.contract)
        || evidence.profile != GateProfile::Full
        || freshness != GateFreshness::Fresh
        || evidence.aggregate != GateAggregate::Passed
        || evidence.dry_run
    {
        return ShipGateStatus::refuse(format!(
            "Forge Gate evidence for `{project_id}` must use a supported contract, the full profile, and be fresh and passed for revision {}; observed contract={}, profile={}, project={}, aggregate={}, freshness={}; run `forge gate --profile full` from the project directory and retry",
            short_revision(revision),
            evidence.contract,
            evidence.profile.label(),
            evidence.project_id,
            evidence.aggregate.label(),
            freshness.label()
        ));
    }
    ShipGateStatus::allow()
}

/// The refusal for a project with no usable Gate evidence.
fn absent(project_id: &str) -> ShipGateStatus {
    ShipGateStatus::refuse(format!(
        "Forge Gate evidence is absent for `{project_id}`; run `forge gate --profile full` from the project directory and retry"
    ))
}

/// The short form of a revision, as the delivery lane renders it.
fn short_revision(revision: &str) -> String {
    revision.chars().take(12).collect()
}
