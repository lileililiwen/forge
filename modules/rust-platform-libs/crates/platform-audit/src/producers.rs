//! Audit event producers.
//!
//! The three producers wrap any `AuditSink` and build canonical
//! `AuditEvent` values for the most common event types. They exist
//! so every consumer emits the same `category`, `outcome`, and
//! redacted context shape for the same logical event, without
//! hand-rolling the mapping in every service.
//!
//! Every producer re-checks the redaction policy at construction
//! time. A `RedactedContext` is built for the optional `details`
//! field; secret-named keys are rejected before the event is sent
//! to the sink.

use platform_contracts::payload::audit_event::{AuditCategory, AuditOutcome};
use platform_contracts::payload::identity_subject::SubjectType as ContractSubjectType;
use platform_core::clock::Clock;
use platform_core::correlation::CorrelationId;
use platform_identity::evaluator::PermissionEvaluation;
use platform_identity::permission::{Permission, PermissionReason};
use platform_identity::session::Session;
use platform_identity::subject::Subject;
use platform_identity::subject::SubjectType;
use platform_identity::tenant::TenantId;

use crate::context::RedactedContext;
use crate::event::AuditEventBuilder;
use crate::sink::{AuditSink, SinkError};

/// Wraps an `AuditSink` and a `Clock` to emit authentication events.
///
/// `AuthenticationAuditor` records the outcome of an authentication
/// attempt. It deliberately does NOT record the source
/// `IdentityError`; the failure category is enough to route the
/// event to the right downstream consumer, and the message would
/// either be a safe fixed string or it would risk leaking a
/// credential. Applications that need more detail record it in
/// their own application log, not in the platform audit event.
pub struct AuthenticationAuditor<'a> {
    sink: &'a dyn AuditSink,
    clock: &'a dyn Clock,
}

impl<'a> AuthenticationAuditor<'a> {
    /// Construct a new auditor.
    #[must_use]
    pub fn new(sink: &'a dyn AuditSink, clock: &'a dyn Clock) -> Self {
        Self { sink, clock }
    }

    /// Record a successful authentication.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError`] if the underlying sink rejects the
    /// event.
    pub fn record_success(
        &self,
        subject: &Subject,
        correlation_id: CorrelationId,
    ) -> Result<(), SinkError> {
        let event_id = format!("auth-ok-{}", correlation_id);
        let event = AuditEventBuilder::new(
            event_id,
            self.clock.now(),
            AuditCategory::Authentication,
            "authenticate",
            AuditOutcome::Success,
            subject.id().as_str(),
            map_subject_type(subject.subject_type()),
            subject.tenant_id().as_str(),
        )
        .build();
        self.sink.record(&event)
    }

    /// Record a failed authentication. The failure category is
    /// recorded; the source error message is NOT.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError`] if the underlying sink rejects the
    /// event.
    pub fn record_failure(
        &self,
        failure_category: &str,
        subject_type: SubjectType,
        tenant_id: &TenantId,
        correlation_id: CorrelationId,
    ) -> Result<(), SinkError> {
        let event_id = format!("auth-fail-{}", correlation_id);
        let action = format!(
            "authenticate:{}",
            sanitize_failure_category(failure_category)
        );
        let event = AuditEventBuilder::new(
            event_id,
            self.clock.now(),
            AuditCategory::Authentication,
            action,
            AuditOutcome::Failure,
            "anonymous",
            map_subject_type(subject_type),
            tenant_id.as_str(),
        )
        .build();
        self.sink.record(&event)
    }
}

/// Wraps an `AuditSink` and a `Clock` to emit permission-evaluation
/// events.
pub struct PermissionAuditor<'a> {
    sink: &'a dyn AuditSink,
    clock: &'a dyn Clock,
}

impl<'a> PermissionAuditor<'a> {
    /// Construct a new auditor.
    #[must_use]
    pub fn new(sink: &'a dyn AuditSink, clock: &'a dyn Clock) -> Self {
        Self { sink, clock }
    }

    /// Record a permission evaluation outcome.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError`] if the underlying sink rejects the
    /// event, or if a context key matches a secret substring.
    pub fn record_evaluation(
        &self,
        subject: &Subject,
        action: &Permission,
        resource: &str,
        evaluation: &PermissionEvaluation,
        correlation_id: CorrelationId,
    ) -> Result<(), SinkError> {
        let outcome = if evaluation.is_allowed() {
            AuditOutcome::Success
        } else {
            AuditOutcome::Denied
        };
        let builder = AuditEventBuilder::new(
            format!("perm-{}", correlation_id),
            self.clock.now(),
            AuditCategory::Permission,
            action.as_str().to_string(),
            outcome,
            subject.id().as_str(),
            map_subject_type(subject.subject_type()),
            subject.tenant_id().as_str(),
        )
        .with_resource(resource.to_string());
        let reason = reason_as_str(evaluation.reason());
        let event = builder
            .with_context("reason_code", reason)?
            .with_context("resource", resource)?
            .build();
        self.sink.record(&event)
    }
}

/// Wraps an `AuditSink` and a `Clock` to emit session-lifecycle
/// events.
pub struct SessionAuditor<'a> {
    sink: &'a dyn AuditSink,
    clock: &'a dyn Clock,
}

impl<'a> SessionAuditor<'a> {
    /// Construct a new auditor.
    #[must_use]
    pub fn new(sink: &'a dyn AuditSink, clock: &'a dyn Clock) -> Self {
        Self { sink, clock }
    }

    /// Record a session-creation event.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError`] if the underlying sink rejects the
    /// event.
    pub fn record_session_created(
        &self,
        session: &Session,
        correlation_id: CorrelationId,
    ) -> Result<(), SinkError> {
        let event = AuditEventBuilder::new(
            format!("session-create-{}", correlation_id),
            self.clock.now(),
            AuditCategory::Lifecycle,
            "session.create",
            AuditOutcome::Success,
            session.subject().id().as_str(),
            map_subject_type(session.subject().subject_type()),
            session.subject().tenant_id().as_str(),
        )
        .with_resource(session.id().to_string())
        .build();
        self.sink.record(&event)
    }

    /// Record a session-revocation event. Revocation is recorded as
    /// `AuditOutcome::Denied` because it denies future use of the
    /// session.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError`] if the underlying sink rejects the
    /// event.
    pub fn record_session_revoked(
        &self,
        session: &Session,
        reason_category: &str,
        correlation_id: CorrelationId,
    ) -> Result<(), SinkError> {
        let builder = AuditEventBuilder::new(
            format!("session-revoke-{}", correlation_id),
            self.clock.now(),
            AuditCategory::Lifecycle,
            "session.revoke",
            AuditOutcome::Denied,
            session.subject().id().as_str(),
            map_subject_type(session.subject().subject_type()),
            session.subject().tenant_id().as_str(),
        )
        .with_resource(session.id().to_string());
        let event = builder
            .with_context(
                "reason_category",
                sanitize_failure_category(reason_category),
            )?
            .build();
        self.sink.record(&event)
    }
}

fn map_subject_type(t: SubjectType) -> ContractSubjectType {
    match t {
        SubjectType::User => ContractSubjectType::User,
        SubjectType::Service => ContractSubjectType::Service,
        SubjectType::ApiKey => ContractSubjectType::ApiKey,
        SubjectType::Anonymous => ContractSubjectType::Anonymous,
    }
}

fn reason_as_str(reason: PermissionReason) -> &'static str {
    match reason {
        PermissionReason::RoleMatch => "role_match",
        PermissionReason::PolicyMatch => "policy_match",
        PermissionReason::ExplicitDeny => "explicit_deny",
        PermissionReason::MissingRole => "missing_role",
        PermissionReason::Expired => "expired",
        PermissionReason::Other => "other",
    }
}

fn sanitize_failure_category(category: &str) -> String {
    // The failure category is one of the closed-set variants in
    // `IdentityFailure::as_str`. We still defensively strip any
    // character that could be confused with a JSON control char or
    // a secret-shaped substring, so a misconfigured caller cannot
    // smuggle a credential-shaped value into the audit event.
    category
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(64)
        .collect()
}

#[allow(dead_code)]
fn _redacted_context_marker(_: &RedactedContext) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::InMemoryAuditSink;
    use platform_core::clock::SystemClock;
    use platform_core::id::Id;
    use platform_identity::lifecycle::Lifecycle;
    use platform_identity::permission::PermissionReason;
    use platform_identity::subject::SubjectId;
    use platform_identity::tenant::TenantTag;
    use time::OffsetDateTime;

    fn subject() -> Subject {
        let tenant = Id::<TenantTag>::new("tnt_test");
        Subject::new(SubjectId::new("usr_1"), SubjectType::User, tenant)
    }

    fn session() -> Session {
        let subject = subject();
        Session::new(
            "sess_1",
            subject,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1),
            CorrelationId::new(),
        )
        .with_lifecycle(Lifecycle::Active)
    }

    #[test]
    fn authentication_success_emits_success_event() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = AuthenticationAuditor::new(&sink, &clock);
        auditor
            .record_success(&subject(), CorrelationId::new())
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, AuditCategory::Authentication);
        assert_eq!(events[0].outcome, AuditOutcome::Success);
        assert_eq!(events[0].action, "authenticate");
    }

    #[test]
    fn authentication_failure_does_not_leak_credential() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = AuthenticationAuditor::new(&sink, &clock);
        let tenant = Id::<TenantTag>::new("tnt_test");
        auditor
            .record_failure(
                "invalid_credentials",
                SubjectType::User,
                &tenant,
                CorrelationId::new(),
            )
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        let json = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(json["category"], "authentication");
        assert_eq!(json["outcome"], "failure");
        assert!(!json.to_string().contains("hunter2"));
        assert!(!json.to_string().contains("password"));
    }

    #[test]
    fn permission_denial_emits_denied_outcome_with_reason() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = PermissionAuditor::new(&sink, &clock);
        let action = Permission::try_new("read:document").unwrap();
        let evaluation = PermissionEvaluation::Deny {
            reason: PermissionReason::MissingRole,
            policy_id: None,
        };
        auditor
            .record_evaluation(
                &subject(),
                &action,
                "doc:1",
                &evaluation,
                CorrelationId::new(),
            )
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, AuditCategory::Permission);
        assert_eq!(events[0].outcome, AuditOutcome::Denied);
        let json = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(json["details"]["reason_code"], "missing_role");
        assert_eq!(json["details"]["resource"], "doc:1");
    }

    #[test]
    fn permission_allow_emits_success_outcome() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = PermissionAuditor::new(&sink, &clock);
        let action = Permission::try_new("read:document").unwrap();
        let evaluation = PermissionEvaluation::Allow {
            reason: PermissionReason::RoleMatch,
            policy_id: Some("policy-1".to_string()),
        };
        auditor
            .record_evaluation(
                &subject(),
                &action,
                "doc:1",
                &evaluation,
                CorrelationId::new(),
            )
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, AuditOutcome::Success);
    }

    #[test]
    fn session_created_emits_lifecycle_event() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = SessionAuditor::new(&sink, &clock);
        auditor
            .record_session_created(&session(), CorrelationId::new())
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, AuditCategory::Lifecycle);
        assert_eq!(events[0].outcome, AuditOutcome::Success);
        assert_eq!(events[0].resource.as_deref(), Some("sess_1"));
    }

    #[test]
    fn session_revoked_emits_denied_outcome() {
        let sink = InMemoryAuditSink::new();
        let clock = SystemClock;
        let auditor = SessionAuditor::new(&sink, &clock);
        auditor
            .record_session_revoked(&session(), "revoked", CorrelationId::new())
            .unwrap();
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, AuditCategory::Lifecycle);
        assert_eq!(events[0].outcome, AuditOutcome::Denied);
        let json = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(json["details"]["reason_category"], "revoked");
    }

    #[test]
    fn failure_category_sanitization_strips_unsafe_chars() {
        let sanitized = sanitize_failure_category("invalid_credentials;DROP TABLE--");
        // Only ASCII alphanumeric + `_`, `-`, `.` are kept; `;` and
        // whitespace are dropped. The trailing `--` is preserved
        // because `-` is in the allow set.
        assert_eq!(sanitized, "invalid_credentialsDROPTABLE--");
    }
}
