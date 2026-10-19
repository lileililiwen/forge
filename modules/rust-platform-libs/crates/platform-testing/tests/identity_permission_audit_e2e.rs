//! End-to-end identity → permission → audit integration test.
//!
//! This test exercises the full flow that the platform supports:
//!
//! 1. A `Subject` is built (representing the resolved identity for
//!    the current request).
//! 2. A `ScriptedPermissionEvaluator` returns a pre-recorded
//!    `PermissionEvaluation` for the call.
//! 3. The `PermissionAuditor` records the evaluation against an
//!    `InMemoryAuditSink`.
//! 4. The test asserts the recorded `AuditEvent` carries the right
//!    `category`, `outcome`, and redacted context, and never
//!    embeds any credential material.
//!
//! The test exists to catch regressions where the platform-side
//! types drift out of sync (e.g. a new field is added to
//! `AuditEvent` but `PermissionAuditor` is not updated). The
//! individual crates have their own unit tests; this test is the
//! cross-crate smoke check.

use platform_audit::sink::InMemoryAuditSink;
use platform_audit::{AuditCategory, AuditOutcome, PermissionAuditor};
use platform_core::clock::SystemClock;
use platform_core::correlation::CorrelationId;
use platform_identity::evaluator::PermissionEvaluator;
use platform_identity::permission::{Permission, PermissionReason};
use platform_identity::subject::{Subject, SubjectType};
use platform_identity::tenant::TenantId;
use platform_testing::ScriptedPermissionEvaluator;
use platform_testing::TestSubject;
use serde_json::json;

#[test]
fn identity_permission_audit_flow_emits_denial_with_redacted_context() {
    // 1. A subject is built.
    let subject: Subject = TestSubject::user("usr_alice").build();

    // 2. The evaluator is wired to deny this specific request.
    let deny = platform_identity::evaluator::PermissionEvaluation::Deny {
        reason: PermissionReason::MissingRole,
        policy_id: None,
    };
    let evaluator = ScriptedPermissionEvaluator::with_script([deny.clone()]);

    // 3. The application evaluates and audits the decision.
    let action = Permission::try_new("read:document").expect("action");
    let sink = InMemoryAuditSink::new();
    let clock = SystemClock;
    let auditor = PermissionAuditor::new(&sink, &clock);
    let correlation_id = CorrelationId::new();

    let resource = "doc:internal-1234";
    let evaluation = evaluator.evaluate(
        &subject,
        &action,
        resource,
        subject.tenant_id(),
        &clock,
        correlation_id,
    );
    assert!(!evaluation.is_allowed());
    auditor
        .record_evaluation(&subject, &action, resource, &evaluation, correlation_id)
        .expect("record");

    // 4. The audit event is present, has the right shape, and
    //    never embeds a credential.
    let events = sink.events();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.category, AuditCategory::Permission);
    assert_eq!(event.outcome, AuditOutcome::Denied);
    assert_eq!(event.action, "read:document");

    let json = serde_json::to_value(event).expect("serialize");
    assert_eq!(json["details"]["reason_code"], "missing_role");
    assert_eq!(json["details"]["resource"], resource);
    // The audit event must never carry a credential-shaped field.
    let rendered = json.to_string().to_lowercase();
    for needle in [
        "password",
        "passwd",
        "token",
        "apikey",
        "api_key",
        "credential",
        "private_key",
        "privatekey",
        "bearer",
        "session_ke",
    ] {
        assert!(
            !rendered.contains(needle),
            "audit event must not contain {needle}; got: {rendered}"
        );
    }
}

#[test]
fn identity_permission_audit_flow_emits_allow_with_resource() {
    let subject: Subject = TestSubject::user("usr_bob").build();
    let allow = platform_identity::evaluator::PermissionEvaluation::Allow {
        reason: PermissionReason::RoleMatch,
        policy_id: Some("policy-1".to_string()),
    };
    let evaluator = ScriptedPermissionEvaluator::with_script([allow.clone()]);
    let action = Permission::try_new("write:document").expect("action");
    let sink = InMemoryAuditSink::new();
    let clock = SystemClock;
    let auditor = PermissionAuditor::new(&sink, &clock);
    let correlation_id = CorrelationId::new();

    let evaluation = evaluator.evaluate(
        &subject,
        &action,
        "doc:1",
        subject.tenant_id(),
        &clock,
        correlation_id,
    );
    assert!(evaluation.is_allowed());
    auditor
        .record_evaluation(&subject, &action, "doc:1", &evaluation, correlation_id)
        .expect("record");

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].outcome, AuditOutcome::Success);
    let json = serde_json::to_value(&events[0]).expect("serialize");
    assert_eq!(json["details"]["reason_code"], "role_match");
    assert_eq!(json["details"]["resource"], "doc:1");
}

#[test]
fn scripted_evaluator_empty_script_falls_back_to_deny_default() {
    let subject: Subject = TestSubject::user("usr_carol").build();
    let action = Permission::try_new("admin:tenant").expect("action");
    let evaluator = ScriptedPermissionEvaluator::new();
    let evaluation = evaluator.evaluate(
        &subject,
        &action,
        "tenant:x",
        subject.tenant_id(),
        &SystemClock,
        CorrelationId::new(),
    );
    assert!(!evaluation.is_allowed());
    assert_eq!(evaluation.reason(), PermissionReason::Other);
    // Sanity: the subject type was preserved end-to-end.
    assert_eq!(subject.subject_type(), SubjectType::User);
    let _ = TenantId::new("tnt_test"); // silence the unused-import lint
    let _ = json!({});
}
