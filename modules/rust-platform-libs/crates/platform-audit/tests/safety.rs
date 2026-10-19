//! Safety tests for `platform-audit`.
//!
//! The design rule is: producers MUST redact secrets before emitting
//! an audit event. The audit crate enforces this at construction time
//! (via [`RedactedContext`]) and via the platform-contracts
//! validator's redaction policy.

use platform_audit::context::RedactedContext;
use platform_audit::event::AuditEventBuilder;
use platform_audit::sink::{AuditSink, InMemoryAuditSink, NoopAuditSink};
use platform_contracts::payload::audit_event::{AuditCategory, AuditOutcome};
use platform_contracts::payload::identity_subject::SubjectType;
use time::OffsetDateTime;

#[test]
fn noop_sink_accepts_any_event() {
    let sink = NoopAuditSink;
    let event = AuditEventBuilder::new(
        "evt_1",
        OffsetDateTime::UNIX_EPOCH,
        AuditCategory::Permission,
        "read:document",
        AuditOutcome::Success,
        "usr_1",
        SubjectType::User,
        "tnt_x",
    )
    .build();
    sink.record(&event).expect("noop sink never fails");
}

#[test]
fn in_memory_sink_collects_events() {
    let sink = InMemoryAuditSink::new();
    assert!(sink.is_empty());

    for i in 0..3 {
        let event = AuditEventBuilder::new(
            format!("evt_{i}"),
            OffsetDateTime::UNIX_EPOCH,
            AuditCategory::DataAccess,
            "read:document",
            AuditOutcome::Success,
            "usr_1",
            SubjectType::User,
            "tnt_x",
        )
        .build();
        sink.record(&event).unwrap();
    }
    assert_eq!(sink.len(), 3);
    let events = sink.events();
    assert_eq!(events[0].event_id, "evt_0");
    assert_eq!(events[2].event_id, "evt_2");
}

#[test]
fn redacted_context_rejects_password_key() {
    let err =
        RedactedContext::from_iter([("password".to_string(), "hunter2".to_string())]).unwrap_err();
    assert_eq!(err.key, "password");
}

#[test]
fn builder_rejects_secret_context_keys() {
    let result = AuditEventBuilder::new(
        "evt_x",
        OffsetDateTime::UNIX_EPOCH,
        AuditCategory::Authentication,
        "login",
        AuditOutcome::Success,
        "usr_x",
        SubjectType::User,
        "tnt_x",
    )
    .with_context("request_id", "req-1")
    .and_then(|b| b.with_context("bearer_token", "leaked"));
    assert!(result.is_err());
}

#[test]
fn builder_emits_only_known_fields() {
    let event = AuditEventBuilder::new(
        "evt_1",
        OffsetDateTime::UNIX_EPOCH,
        AuditCategory::Permission,
        "read:document",
        AuditOutcome::Success,
        "usr_1",
        SubjectType::User,
        "tnt_x",
    )
    .with_context("request_id", "req-1")
    .expect("non-secret context")
    .build();
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_id"], "evt_1");
    assert_eq!(json["category"], "permission");
    assert_eq!(json["outcome"], "success");
    assert!(json["details"]["request_id"].is_string());
    // No accidental secret-shaped keys
    assert!(json["details"].get("password").is_none());
    assert!(json["details"].get("token").is_none());
}
