//! Minimal `platform-audit` usage: build an event and record it.
//!
//! This example is the compile-checked companion to the
//! `platform-audit` section of `docs/usage.md`. It exercises only the
//! crate's public API and adds no new dependencies.

use platform_audit::{
    sink::InMemoryAuditSink, AuditCategory, AuditEventBuilder, AuditOutcome, AuditSink,
    AuditSubjectType,
};
use platform_core::{Clock, FixedClock};

fn main() {
    let clock = FixedClock::at_epoch();
    let sink = InMemoryAuditSink::new();

    // The builder stamps the event; redaction-checked context rejects
    // secret-named keys at construction.
    let event = AuditEventBuilder::new(
        "evt-1",
        clock.now(),
        AuditCategory::Authentication,
        "authenticate",
        AuditOutcome::Success,
        "usr_alice",
        AuditSubjectType::User,
        "acme",
    )
    .with_resource("session-7")
    .with_context("ip", "203.0.113.10")
    .expect("non-secret context key accepted")
    .build();

    sink.record(&event).expect("in-memory sink accepts");
    assert_eq!(sink.len(), 1);
    println!("recorded {} event(s)", sink.len());
}
