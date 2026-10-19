//! Permission evaluator trait and reference implementations.
//!
//! The trait is the application-owned side of the authorization
//! decision. The platform exposes only the trait shape, the result
//! type, and two reference implementations (`DenyByDefaultEvaluator`
//! and `AllowListEvaluator`). The application plugs in its real
//! policy: role-based, attribute-based, policy-as-code, or whatever
//! fits.
//!
//! All implementations are synchronous. Asynchronous adapters wrap
//! the call in a runtime of their choice; the foundation does not
//! depend on a specific executor.

use std::collections::HashSet;
use std::sync::RwLock;

use platform_core::clock::Clock;
use platform_core::correlation::CorrelationId;

use crate::permission::{Permission, PermissionReason};
use crate::subject::{Subject, SubjectId};
use crate::tenant::TenantId;

/// Outcome of a permission evaluation.
///
/// Evaluation is a discriminated union (not a `Result`) because
/// authorization never fails in the `Result` sense — the policy
/// either matched or did not. The reason code is mandatory; the
/// policy id is optional (some consumers do not have policy ids).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PermissionEvaluation {
    /// Authorization granted.
    Allow {
        /// Reason the policy matched.
        reason: PermissionReason,
        /// Optional policy identifier the application wants to record.
        policy_id: Option<String>,
    },
    /// Authorization denied.
    Deny {
        /// Reason the policy did not match (or explicitly denied).
        reason: PermissionReason,
        /// Optional policy identifier that produced the denial.
        policy_id: Option<String>,
    },
}

impl PermissionEvaluation {
    /// Returns true iff the evaluation is `Allow`.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    /// Returns the reason code.
    #[must_use]
    pub fn reason(&self) -> PermissionReason {
        match self {
            Self::Allow { reason, .. } | Self::Deny { reason, .. } => *reason,
        }
    }
}

/// Application-implemented permission evaluator.
///
/// Implementations MUST be deterministic for a given
/// `(subject, action, resource, tenant)` tuple when the underlying
/// policy is deterministic. The platform does not require
/// determinism — some consumers need time-of-day or risk-based
/// policies — but the trait shape does not depend on it.
pub trait PermissionEvaluator: Send + Sync {
    /// Evaluate the permission.
    fn evaluate(
        &self,
        subject: &Subject,
        action: &Permission,
        resource: &str,
        tenant: &TenantId,
        clock: &dyn Clock,
        correlation_id: CorrelationId,
    ) -> PermissionEvaluation;
}

/// Evaluator that denies every request. Useful as a safe default
/// during wiring, in tests that exercise the denial path, and as
/// the fallback for [`super::ScriptedPermissionEvaluator`] when its
/// script is exhausted.
#[derive(Debug, Default, Clone, Copy)]
pub struct DenyByDefaultEvaluator;

impl PermissionEvaluator for DenyByDefaultEvaluator {
    fn evaluate(
        &self,
        _subject: &Subject,
        _action: &Permission,
        _resource: &str,
        _tenant: &TenantId,
        _clock: &dyn Clock,
        _correlation_id: CorrelationId,
    ) -> PermissionEvaluation {
        PermissionEvaluation::Deny {
            reason: PermissionReason::Other,
            policy_id: None,
        }
    }
}

/// Evaluator that allows only listed `(SubjectId, Permission)` pairs.
/// Unknown pairs default to deny with `PermissionReason::MissingRole`.
///
/// The set is held behind an `RwLock` so an application can mutate
/// it at runtime (e.g. after a role assignment) without taking
/// `&mut` on the whole evaluator.
#[derive(Debug, Default)]
pub struct AllowListEvaluator {
    allow: RwLock<HashSet<(SubjectId, Permission)>>,
}

impl AllowListEvaluator {
    /// Construct an empty allow list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            allow: RwLock::new(HashSet::new()),
        }
    }

    /// Construct an allow list pre-populated with the given pairs.
    #[must_use]
    pub fn with_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (SubjectId, Permission)>,
    {
        let mut set = HashSet::new();
        for pair in pairs {
            set.insert(pair);
        }
        Self {
            allow: RwLock::new(set),
        }
    }

    /// Grant a `(SubjectId, Permission)` pair.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionError::Poisoned`] if the underlying
    /// `RwLock` is poisoned (a panic while a writer held the lock).
    pub fn grant(&self, subject: SubjectId, action: Permission) -> Result<(), PermissionError> {
        let mut guard = self.allow.write().map_err(|_| PermissionError::Poisoned)?;
        guard.insert((subject, action));
        Ok(())
    }

    /// Revoke a `(SubjectId, Permission)` pair.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionError::Poisoned`] if the underlying
    /// `RwLock` is poisoned.
    pub fn revoke(&self, subject: &SubjectId, action: &Permission) -> Result<(), PermissionError> {
        let mut guard = self.allow.write().map_err(|_| PermissionError::Poisoned)?;
        guard.remove(&(subject.clone(), action.clone()));
        Ok(())
    }
}

impl PermissionEvaluator for AllowListEvaluator {
    fn evaluate(
        &self,
        subject: &Subject,
        action: &Permission,
        _resource: &str,
        _tenant: &TenantId,
        _clock: &dyn Clock,
        _correlation_id: CorrelationId,
    ) -> PermissionEvaluation {
        let key = (subject.id().clone(), action.clone());
        let allowed = self
            .allow
            .read()
            .map_err(|_| ())
            .map(|guard| guard.contains(&key))
            .unwrap_or(false);
        if allowed {
            PermissionEvaluation::Allow {
                reason: PermissionReason::RoleMatch,
                policy_id: None,
            }
        } else {
            PermissionEvaluation::Deny {
                reason: PermissionReason::MissingRole,
                policy_id: None,
            }
        }
    }
}

/// Errors raised by the platform-side permission evaluators.
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    /// The internal lock was poisoned (a panic while a writer held
    /// the lock). The platform never holds the lock across a panic
    /// itself; this is raised only when an application thread
    /// panics while holding the evaluator.
    #[error("permission evaluator internal lock poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subject::{Subject, SubjectType};
    use crate::tenant::TenantTag;
    use platform_core::clock::SystemClock;
    use platform_core::id::Id;

    fn subject(id: &str) -> Subject {
        let tenant = Id::<TenantTag>::new("tnt_test");
        Subject::new(SubjectId::new(id), SubjectType::User, tenant)
    }

    fn action(name: &str) -> Permission {
        Permission::try_new(name).expect("action")
    }

    fn tenant_id() -> TenantId {
        Id::<TenantTag>::new("tnt_test")
    }

    fn clock() -> SystemClock {
        SystemClock
    }

    #[test]
    fn deny_by_default_evaluator_denies_everything() {
        let evaluator = DenyByDefaultEvaluator;
        let s = subject("usr_1");
        let a = action("read:document");
        let result = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            &tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(
            result,
            PermissionEvaluation::Deny {
                reason: PermissionReason::Other,
                policy_id: None,
            }
        );
    }

    #[test]
    fn allow_list_returns_allow_for_listed_pair() {
        let s_id = SubjectId::new("usr_1");
        let a = action("read:document");
        let evaluator = AllowListEvaluator::with_pairs([(s_id.clone(), a.clone())]);
        let s = subject("usr_1");
        let result = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            &tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(
            result,
            PermissionEvaluation::Allow {
                reason: PermissionReason::RoleMatch,
                policy_id: None,
            }
        );
    }

    #[test]
    fn allow_list_returns_deny_for_unlisted_pair() {
        let evaluator = AllowListEvaluator::new();
        let s = subject("usr_1");
        let a = action("read:document");
        let result = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            &tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert_eq!(
            result,
            PermissionEvaluation::Deny {
                reason: PermissionReason::MissingRole,
                policy_id: None,
            }
        );
    }

    #[test]
    fn allow_list_grant_and_revoke_round_trip() {
        let evaluator = AllowListEvaluator::new();
        let s_id = SubjectId::new("usr_1");
        let a = action("read:document");
        evaluator.grant(s_id.clone(), a.clone()).expect("grant");
        let s = subject("usr_1");
        let allowed = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            &tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert!(allowed.is_allowed());
        evaluator.revoke(&s_id, &a).expect("revoke");
        let denied = evaluator.evaluate(
            &s,
            &a,
            "doc:1",
            &tenant_id(),
            &clock(),
            CorrelationId::new(),
        );
        assert!(!denied.is_allowed());
    }

    #[test]
    fn evaluation_is_serialized_with_decision_tag() {
        let eval = PermissionEvaluation::Allow {
            reason: PermissionReason::RoleMatch,
            policy_id: Some("policy-1".to_string()),
        };
        let json = serde_json::to_value(&eval).unwrap();
        assert_eq!(json["decision"], "allow");
        assert_eq!(json["reason"], "role_match");
        assert_eq!(json["policy_id"], "policy-1");
    }
}
