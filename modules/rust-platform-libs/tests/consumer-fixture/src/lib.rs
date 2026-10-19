//! Conformance fixture: the smallest possible in-workspace consumer.
//!
//! This crate exists so `scripts/conformance.sh` can prove that a
//! consumer following the documented recipe resolves the platform
//! crates, builds, and gets passing tests under the pinned toolchain.
//! It exercises the documented path-dependency form from
//! `docs/consumer-baseline.md` and nothing else: no git, registry, or
//! sibling-relative hacks.
//!
//! The test below wires the same identity → permission → audit path a
//! real consumer uses: a `TestSubject`, the `DenyByDefaultEvaluator`,
//! and a `PermissionAuditor` recording into an `InMemoryAuditSink`.

#[cfg(test)]
mod tests {
    use platform_audit::sink::InMemoryAuditSink;
    use platform_audit::{AuditCategory, AuditOutcome, PermissionAuditor};
    use platform_core::{CorrelationId, FixedClock};
    use platform_identity::{DenyByDefaultEvaluator, Permission, PermissionEvaluator};
    use platform_testing::TestSubject;

    #[test]
    fn deny_by_default_end_to_end() {
        let clock = FixedClock::at_epoch();
        let sink = InMemoryAuditSink::new();
        let auditor = PermissionAuditor::new(&sink, &clock);

        let subject = TestSubject::user("alice").in_tenant("acme").build();
        let action = Permission::try_new("billing.read").expect("fixture action is within bounds");
        let evaluation = DenyByDefaultEvaluator.evaluate(
            &subject,
            &action,
            "invoice-42",
            subject.tenant_id(),
            &clock,
            CorrelationId::new(),
        );
        assert!(!evaluation.is_allowed());

        auditor
            .record_evaluation(
                &subject,
                &action,
                "invoice-42",
                &evaluation,
                CorrelationId::new(),
            )
            .expect("in-memory sink accepts the event");

        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, AuditCategory::Permission);
        assert_eq!(events[0].outcome, AuditOutcome::Denied);
    }
}
