//! Scripted permission evaluator for tests.
//!
//! `ScriptedPermissionEvaluator` returns the next pre-recorded
//! `PermissionEvaluation` in FIFO order, regardless of the
//! `subject`, `action`, or `resource`. This lets a test assert that
//! a service under test wires the right number of `evaluate` calls
//! and feeds the results to its downstream consumers (audit
//! producer, error mapper, ...) in the right order.
//!
//! When the script is exhausted, the evaluator falls back to a
//! configurable default (defaults to `DenyByDefaultEvaluator`).
//!
//! This crate is test-only. Production crates MUST NOT depend on
//! `platform-testing`; the architecture test
//! (`platform-arch-tests`) enforces that direction.

use std::sync::Mutex;

use platform_core::clock::Clock;
use platform_core::correlation::CorrelationId;
use platform_identity::evaluator::{
    DenyByDefaultEvaluator, PermissionEvaluation, PermissionEvaluator,
};
use platform_identity::permission::Permission;
use platform_identity::subject::Subject;
use platform_identity::tenant::TenantId;

/// Scripted FIFO evaluator for tests.
///
/// Each [`evaluate`](PermissionEvaluator::evaluate) call returns the
/// next pre-recorded outcome. When the script is exhausted, the
/// fallback is consulted (defaults to
/// [`DenyByDefaultEvaluator`]).
pub struct ScriptedPermissionEvaluator {
    script: Mutex<Vec<PermissionEvaluation>>,
    fallback: Box<dyn PermissionEvaluator>,
}

impl std::fmt::Debug for ScriptedPermissionEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.script.lock().map_err(|_| std::fmt::Error)?;
        f.debug_struct("ScriptedPermissionEvaluator")
            .field("script_length", &guard.len())
            .field("fallback", &"<dyn PermissionEvaluator>")
            .finish()
    }
}

impl ScriptedPermissionEvaluator {
    /// Construct a scripted evaluator with an empty script and the
    /// default fallback (`DenyByDefaultEvaluator`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            script: Mutex::new(Vec::new()),
            fallback: Box::new(DenyByDefaultEvaluator),
        }
    }

    /// Construct a scripted evaluator pre-populated with the given
    /// script and the default fallback.
    #[must_use]
    pub fn with_script<I>(script: I) -> Self
    where
        I: IntoIterator<Item = PermissionEvaluation>,
    {
        Self {
            script: Mutex::new(script.into_iter().collect()),
            fallback: Box::new(DenyByDefaultEvaluator),
        }
    }

    /// Construct a scripted evaluator with the given script and a
    /// custom fallback. Use this when the test needs the fallback
    /// to be an `AllowListEvaluator` (or any other application-side
    /// implementation).
    #[must_use]
    pub fn with_script_and_fallback<I>(script: I, fallback: Box<dyn PermissionEvaluator>) -> Self
    where
        I: IntoIterator<Item = PermissionEvaluation>,
    {
        Self {
            script: Mutex::new(script.into_iter().collect()),
            fallback,
        }
    }

    /// Append an outcome to the script. Useful for tests that
    /// build the script incrementally rather than as a single
    /// literal.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptedEvaluatorError::Poisoned`] if the internal
    /// `Mutex` is poisoned.
    pub fn push(&self, evaluation: PermissionEvaluation) -> Result<(), ScriptedEvaluatorError> {
        let mut guard = self
            .script
            .lock()
            .map_err(|_| ScriptedEvaluatorError::Poisoned)?;
        guard.push(evaluation);
        Ok(())
    }

    /// Number of outcomes remaining in the script.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.script.lock().map(|g| g.len()).unwrap_or(0)
    }
}

impl Default for ScriptedPermissionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionEvaluator for ScriptedPermissionEvaluator {
    fn evaluate(
        &self,
        subject: &Subject,
        action: &Permission,
        resource: &str,
        tenant: &TenantId,
        clock: &dyn Clock,
        correlation_id: CorrelationId,
    ) -> PermissionEvaluation {
        // Try the script first; fall back when empty or poisoned.
        if let Ok(mut guard) = self.script.lock() {
            if let Some(next) = guard.first().cloned() {
                guard.remove(0);
                return next;
            }
        }
        self.fallback
            .evaluate(subject, action, resource, tenant, clock, correlation_id)
    }
}

/// Errors raised by the scripted evaluator.
#[derive(Debug, thiserror::Error)]
pub enum ScriptedEvaluatorError {
    /// The internal `Mutex` was poisoned (a panic while a writer
    /// held the lock).
    #[error("scripted evaluator internal mutex poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subject::TestSubject;
    use platform_identity::permission::PermissionReason;

    fn subject() -> Subject {
        TestSubject::user("usr_1").build()
    }

    fn action() -> Permission {
        Permission::try_new("read:document").expect("action")
    }

    fn clock() -> platform_core::clock::SystemClock {
        platform_core::clock::SystemClock
    }

    #[test]
    fn empty_script_falls_back_to_deny() {
        let evaluator = ScriptedPermissionEvaluator::new();
        let s = subject();
        let a = action();
        let result = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert!(!result.is_allowed());
    }

    #[test]
    fn script_drains_in_fifo_order() {
        let allow = PermissionEvaluation::Allow {
            reason: PermissionReason::RoleMatch,
            policy_id: None,
        };
        let deny = PermissionEvaluation::Deny {
            reason: PermissionReason::MissingRole,
            policy_id: None,
        };
        let evaluator = ScriptedPermissionEvaluator::with_script([allow.clone(), deny.clone()]);
        let s = subject();
        let a = action();
        let first = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(first, allow);
        let second = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(second, deny);
        // Third call falls back to deny.
        let third = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert!(!third.is_allowed());
    }

    #[test]
    fn push_appends_to_script() {
        let evaluator = ScriptedPermissionEvaluator::new();
        let allow = PermissionEvaluation::Allow {
            reason: PermissionReason::RoleMatch,
            policy_id: None,
        };
        evaluator.push(allow.clone()).expect("push");
        assert_eq!(evaluator.remaining(), 1);
        let s = subject();
        let a = action();
        let result = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(result, allow);
        assert_eq!(evaluator.remaining(), 0);
    }

    #[test]
    fn custom_fallback_is_used_after_script_exhausts() {
        let allow = PermissionEvaluation::Allow {
            reason: PermissionReason::RoleMatch,
            policy_id: None,
        };
        let fallback = DenyByDefaultEvaluator;
        let evaluator = ScriptedPermissionEvaluator::with_script_and_fallback(
            [allow.clone()],
            Box::new(fallback),
        );
        let s = subject();
        let a = action();
        let first = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(first, allow);
        // After exhaustion, the fallback denies.
        let second = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            s.tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert!(!second.is_allowed());
    }
}
