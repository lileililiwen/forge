//! Fleet publish queue state.
//!
//! A *queue* is one fleet invocation: one or more eligible projects
//! published sequentially through one external provider. Each queue
//! carries a stable `queue_id`; every project invocation inside the
//! queue records one `publish` journal row tagged with that id so
//! `forge deploy status` can answer fleet-wide, project-specific and
//! queue-specific queries without contacting a provider.
//!
//! Invariants enforced by [`QueueState::start_project`]:
//!
//! 1. At most one project is in the `running` state at any time.
//! 2. A terminal project never returns to `running`.
//! 3. Duplicate terminal events for the same `operation_id` are
//!    ignored — the first terminal state wins.
//! 4. The queue_id is bound to a single provider name so the
//!    progress-event transport can refuse cross-queue spoofing.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Bounded, public state vocabulary for a single project slot inside
/// a fleet queue. The values are the exact strings persisted to the
/// journal and surfaced through `forge deploy status`; never widen
/// the set without also updating the CLI renderers and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Queued,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

impl fmt::Display for ProjectState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ProjectState {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectState::Queued => "queued",
            ProjectState::Running => "running",
            ProjectState::Succeeded => "succeeded",
            ProjectState::Failed => "failed",
            ProjectState::TimedOut => "timed_out",
            ProjectState::Cancelled => "cancelled",
        }
    }

    /// Terminal states never return to `running` or `queued`. Used by
    /// [`QueueState::start_project`] to reject replay attempts.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            ProjectState::Succeeded
                | ProjectState::Failed
                | ProjectState::TimedOut
                | ProjectState::Cancelled
        )
    }
}

/// Per-project slot in a fleet queue. The `provider` and `revision`
/// fields are denormalized so the status projection never has to
/// join against external state — every question the operator asks
/// through `forge deploy status` answers from this record alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueProject {
    pub project_id: String,
    pub operation_id: String,
    pub provider: String,
    pub revision: String,
    pub state: ProjectState,
    /// Most recent bounded detail string from the provider (already
    /// passed through `policy::redact_credentials`).
    #[serde(default)]
    pub detail: Option<String>,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
}

impl QueueProject {
    pub fn new(
        project_id: impl Into<String>,
        operation_id: impl Into<String>,
        provider: impl Into<String>,
        revision: impl Into<String>,
        started_at: impl Into<String>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            operation_id: operation_id.into(),
            provider: provider.into(),
            revision: revision.into(),
            state: ProjectState::Queued,
            detail: None,
            started_at: started_at.into(),
            finished_at: None,
        }
    }
}

/// One fleet queue. Forge creates exactly one per fleet invocation;
/// the queue is the durable source for the per-project terminal
/// states that drive `forge deploy status --queue <id>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueState {
    pub queue_id: String,
    pub provider: String,
    pub created_at: String,
    pub projects: Vec<QueueProject>,
}

impl QueueState {
    pub fn new(
        queue_id: impl Into<String>,
        provider: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            queue_id: queue_id.into(),
            provider: provider.into(),
            created_at: created_at.into(),
            projects: Vec::new(),
        }
    }

    pub fn push_queued(&mut self, project: QueueProject) {
        self.projects.push(project);
    }

    pub fn project(&self, project_id: &str) -> Option<&QueueProject> {
        self.projects.iter().find(|p| p.project_id == project_id)
    }

    /// Move one queued project into [`ProjectState::Running`]. Refuses
    /// if the project is already terminal or if any *other* project is
    /// currently running — the one-running invariant is enforced
    /// across the whole queue.
    pub fn start_project(&mut self, project_id: &str) -> Result<(), QueueTransitionError> {
        if let Some(other) = self
            .projects
            .iter()
            .find(|p| p.state == ProjectState::Running && p.project_id != project_id)
        {
            return Err(QueueTransitionError::OtherRunning(other.project_id.clone()));
        }
        let target = self
            .projects
            .iter_mut()
            .find(|p| p.project_id == project_id)
            .ok_or_else(|| QueueTransitionError::UnknownProject(project_id.to_string()))?;
        if target.state.is_terminal() {
            return Err(QueueTransitionError::AlreadyTerminal {
                project: project_id.to_string(),
                state: target.state,
            });
        }
        target.state = ProjectState::Running;
        target.detail = None;
        Ok(())
    }

    /// Record a terminal state. Duplicate terminal events for the same
    /// operation_id are silently ignored — the first terminal state
    /// wins so a noisy provider cannot flip a project from
    /// `succeeded` to `failed` after the fact.
    pub fn finalize_project(
        &mut self,
        operation_id: &str,
        state: ProjectState,
        detail: Option<String>,
        finished_at: &str,
    ) -> Result<(), QueueTransitionError> {
        if !state.is_terminal() {
            return Err(QueueTransitionError::NonTerminal(state));
        }
        let target = self
            .projects
            .iter_mut()
            .find(|p| p.operation_id == operation_id)
            .ok_or_else(|| QueueTransitionError::UnknownOperation(operation_id.to_string()))?;
        if target.state.is_terminal() {
            return Ok(());
        }
        target.state = state;
        target.detail = detail;
        target.finished_at = Some(finished_at.to_string());
        Ok(())
    }

    /// Project currently in the running slot, if any. Used by the
    /// status projection so a fleet report names exactly one active
    /// project (or reports `none` when the queue is idle).
    pub fn active_project(&self) -> Option<&QueueProject> {
        self.projects
            .iter()
            .find(|p| p.state == ProjectState::Running)
    }

    /// Counts of every terminal state plus the still-active count.
    /// Used by the aggregate summary at the bottom of
    /// `forge deploy status`.
    pub fn aggregate(&self) -> QueueAggregate {
        let mut agg = QueueAggregate::default();
        for project in &self.projects {
            match project.state {
                ProjectState::Queued => agg.queued += 1,
                ProjectState::Running => agg.running += 1,
                ProjectState::Succeeded => agg.succeeded += 1,
                ProjectState::Failed => agg.failed += 1,
                ProjectState::TimedOut => agg.timed_out += 1,
                ProjectState::Cancelled => agg.cancelled += 1,
            }
        }
        agg
    }

    pub fn is_terminal(&self) -> bool {
        self.projects.iter().all(|p| p.state.is_terminal())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueAggregate {
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub cancelled: usize,
}

impl QueueAggregate {
    pub fn total(&self) -> usize {
        self.queued + self.running + self.succeeded + self.failed + self.timed_out + self.cancelled
    }
    /// `true` only when every project is in the `succeeded` bucket.
    /// Drives the `--watch` exit code and the fleet-wide success exit.
    pub fn all_succeeded(&self) -> bool {
        self.total() > 0
            && self.succeeded == self.total()
            && self.failed == 0
            && self.timed_out == 0
            && self.cancelled == 0
            && self.running == 0
            && self.queued == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueTransitionError {
    /// Another project is already in `running`; the queue enforces a
    /// single-active invariant.
    OtherRunning(String),
    /// Project id was never added to this queue.
    UnknownProject(String),
    /// Operation id never matched a project slot.
    UnknownOperation(String),
    /// Project has already reached a terminal state and cannot move
    /// back to `running`.
    AlreadyTerminal {
        project: String,
        state: ProjectState,
    },
    /// Finalize was called with a non-terminal state.
    NonTerminal(ProjectState),
}

impl std::fmt::Display for QueueTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueTransitionError::OtherRunning(other) => {
                write!(f, "another project `{other}` is already running")
            }
            QueueTransitionError::UnknownProject(project) => {
                write!(f, "project `{project}` is not in this queue")
            }
            QueueTransitionError::UnknownOperation(op) => {
                write!(f, "operation id `{op}` is not in this queue")
            }
            QueueTransitionError::AlreadyTerminal { project, state } => write!(
                f,
                "project `{project}` already reached terminal state `{state}`"
            ),
            QueueTransitionError::NonTerminal(state) => {
                write!(f, "state `{state}` is not terminal")
            }
        }
    }
}

impl std::error::Error for QueueTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_with(projects: &[(&str, &str)]) -> QueueState {
        let mut q = QueueState::new("fleet-1", "jenkins", "2026-09-27T00:00:00Z");
        for (i, (id, op)) in projects.iter().enumerate() {
            q.push_queued(QueueProject::new(
                *id,
                *op,
                "jenkins",
                "abc",
                format!("2026-09-27T00:00:0{i}Z"),
            ));
        }
        q
    }

    #[test]
    fn start_running_then_finalize_succeeds() {
        let mut q = queue_with(&[("alpha", "publish-alpha-ab"), ("beta", "publish-beta-cd")]);
        q.start_project("alpha").unwrap();
        assert_eq!(q.active_project().unwrap().project_id, "alpha");
        q.finalize_project(
            "publish-alpha-ab",
            ProjectState::Succeeded,
            None,
            "2026-09-27T00:00:10Z",
        )
        .unwrap();
        assert_eq!(q.project("alpha").unwrap().state, ProjectState::Succeeded);
        assert!(q.active_project().is_none());
        assert!(!q.is_terminal());
        q.start_project("beta").unwrap();
        q.finalize_project(
            "publish-beta-cd",
            ProjectState::Succeeded,
            None,
            "2026-09-27T00:01:10Z",
        )
        .unwrap();
        assert!(q.is_terminal());
    }

    #[test]
    fn second_running_project_refused() {
        let mut q = queue_with(&[("alpha", "op-a"), ("beta", "op-b")]);
        q.start_project("alpha").unwrap();
        let err = q.start_project("beta").unwrap_err();
        assert!(matches!(err, QueueTransitionError::OtherRunning(ref id) if id == "alpha"));
    }

    #[test]
    fn terminal_state_blocks_rerun() {
        let mut q = queue_with(&[("alpha", "op-a")]);
        q.start_project("alpha").unwrap();
        q.finalize_project("op-a", ProjectState::Failed, None, "t")
            .unwrap();
        let err = q.start_project("alpha").unwrap_err();
        assert!(matches!(err, QueueTransitionError::AlreadyTerminal { .. }));
    }

    #[test]
    fn duplicate_terminal_is_ignored() {
        let mut q = queue_with(&[("alpha", "op-a")]);
        q.start_project("alpha").unwrap();
        q.finalize_project("op-a", ProjectState::Succeeded, None, "t1")
            .unwrap();
        // Second terminal event for the same operation_id is a no-op.
        q.finalize_project("op-a", ProjectState::Failed, None, "t2")
            .unwrap();
        assert_eq!(q.project("alpha").unwrap().state, ProjectState::Succeeded);
        assert_eq!(
            q.project("alpha").unwrap().finished_at.as_deref(),
            Some("t1")
        );
    }

    #[test]
    fn non_terminal_finalize_refused() {
        let mut q = queue_with(&[("alpha", "op-a")]);
        q.start_project("alpha").unwrap();
        let err = q
            .finalize_project("op-a", ProjectState::Running, None, "t")
            .unwrap_err();
        assert!(matches!(err, QueueTransitionError::NonTerminal(_)));
    }

    #[test]
    fn aggregate_counts_every_state() {
        let mut q = queue_with(&[("a", "op-a"), ("b", "op-b"), ("c", "op-c"), ("d", "op-d")]);
        q.start_project("a").unwrap();
        q.finalize_project("op-b", ProjectState::Succeeded, None, "t")
            .unwrap();
        q.finalize_project("op-c", ProjectState::TimedOut, None, "t")
            .unwrap();
        let agg = q.aggregate();
        assert_eq!(agg.running, 1);
        assert_eq!(agg.queued, 1);
        assert_eq!(agg.succeeded, 1);
        assert_eq!(agg.timed_out, 1);
        assert!(!agg.all_succeeded());
    }
}
