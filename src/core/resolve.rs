//! The one rule that names what a resolution did.
//!
//! A resolution reads a catalog, picks the strongest compatible
//! candidates for a profile and returns a **plan**. It installs
//! nothing: no file is written, no source changes, no installation
//! runs. The journal state and the command output therefore name the
//! run a resolution describes, and a plan is never a completed
//! resolution.
//!
//! Both resolver lanes — the semantic component registry and the
//! semantic UI pattern catalog — read this one rule, so the two
//! cannot drift apart.

/// What a resolution run did.
///
/// The journal state names this and nothing else: a plan is `planned`,
/// never `done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveRun {
    /// A plan the resolver produced.
    Planned,
    /// A request the resolver could not satisfy.
    Rejected,
}

impl ResolveRun {
    /// The run a resolution describes. A resolution with at least one
    /// step is a planned run; a request the resolver could not
    /// satisfy is a rejected run.
    pub fn for_steps(step_count: usize) -> Self {
        if step_count == 0 {
            ResolveRun::Rejected
        } else {
            ResolveRun::Planned
        }
    }

    /// The registry journal state this run records. A planned
    /// resolution is a plan, a rejected resolution is a refusal.
    pub fn journal_state(self) -> &'static str {
        match self {
            ResolveRun::Planned => "planned",
            ResolveRun::Rejected => "rejected",
        }
    }

    /// The label the journal detail and the command output publish, so
    /// a reader never has to guess what the state means.
    pub fn label(self) -> &'static str {
        match self {
            ResolveRun::Planned => "planned",
            ResolveRun::Rejected => "rejected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResolveRun;

    #[test]
    fn a_plan_is_never_a_completed_resolution() {
        assert_eq!(ResolveRun::for_steps(1).journal_state(), "planned");
        assert_eq!(
            ResolveRun::for_steps(1).label(),
            "planned",
            "a resolution installs nothing, so it is never `done`"
        );
    }

    #[test]
    fn a_resolution_that_satisfied_nothing_is_rejected() {
        assert_eq!(ResolveRun::for_steps(0).journal_state(), "rejected");
        assert_eq!(ResolveRun::for_steps(0).label(), "rejected");
    }

    #[test]
    fn every_run_names_itself_in_the_detail() {
        assert_eq!(ResolveRun::Planned.label(), "planned");
        assert_eq!(ResolveRun::Rejected.label(), "rejected");
    }
}
