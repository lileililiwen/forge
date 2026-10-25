//! The one rule for the state of an API operation row.
//!
//! Every HTTP request that runs through the shared operation boundary
//! leaves one row in the registry journal. The row's state names what
//! the operation did: a completed operation did the work, a planned
//! operation only planned, a blocked operation was refused before it
//! changed anything and a failed operation did not finish.
//!
//! The boundary reads this rule. No route writes a state of its own,
//! because a state a reader has to trust is not a rule they can read.

use crate::deploy::DeployVerdict;

/// What an operation row earned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationOutcome {
    /// The operation did the work it was asked to do.
    Completed,
    /// The operation only planned: nothing was applied.
    Planned,
    /// The shared rule refused the operation before it changed anything.
    Blocked,
    /// The operation did not finish.
    Failed,
}

impl OperationOutcome {
    /// The state the journal row records.
    pub fn journal_state(self) -> &'static str {
        match self {
            OperationOutcome::Completed => "done",
            OperationOutcome::Planned => "planned",
            OperationOutcome::Blocked => "blocked",
            OperationOutcome::Failed => "failed",
        }
    }

    /// The detail the journal row carries. The detail names the run, so
    /// a reader never has to guess what the state means.
    pub fn detail(self, kind: &str) -> String {
        match self {
            OperationOutcome::Completed => format!("{kind} completed"),
            OperationOutcome::Planned => format!("{kind} planned"),
            OperationOutcome::Blocked => format!("{kind} blocked"),
            OperationOutcome::Failed => format!("{kind} failed"),
        }
    }

    /// The outcome of a deploy run, read from the deploy model's own
    /// verdict. The deploy rule judges the run; this rule only names the
    /// row's state, so the two never disagree about a deploy.
    pub fn from_deploy_verdict(verdict: DeployVerdict) -> Self {
        match verdict {
            DeployVerdict::Delivered => OperationOutcome::Completed,
            DeployVerdict::Planned => OperationOutcome::Planned,
            DeployVerdict::Blocked => OperationOutcome::Blocked,
            DeployVerdict::Failed => OperationOutcome::Failed,
        }
    }
}
