//! Minimal `platform-identity` usage: a subject plus a permission check.
//!
//! This example is the compile-checked companion to the
//! `platform-identity` section of `docs/usage.md`. It exercises only
//! the crate's public API and adds no new dependencies.

use platform_core::{CorrelationId, FixedClock};
use platform_identity::{
    DenyByDefaultEvaluator, Permission, PermissionEvaluator, Subject, SubjectId, SubjectType,
    TenantId,
};

fn main() {
    // The application resolves a subject from its own store; the
    // platform only sees the resulting trait shape.
    let subject = Subject::new(
        SubjectId::new("usr_alice"),
        SubjectType::User,
        TenantId::new("acme"),
    );

    // Permission keys are bounded strings mirroring the schema.
    let action = Permission::try_new("billing.read").expect("action fits bounds");

    // Reference evaluators are safe defaults, not production policy:
    // deny-by-default rejects everything.
    let clock = FixedClock::at_epoch();
    let evaluation = DenyByDefaultEvaluator.evaluate(
        &subject,
        &action,
        "invoice-42",
        subject.tenant_id(),
        &clock,
        CorrelationId::new(),
    );
    assert!(!evaluation.is_allowed());
    println!("decision: denied ({:?})", evaluation.reason());
}
