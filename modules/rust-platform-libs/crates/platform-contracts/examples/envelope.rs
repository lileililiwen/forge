//! Minimal `platform-contracts` usage: wrap a payload in an envelope.
//!
//! This example is the compile-checked companion to the
//! `platform-contracts` section of `docs/usage.md`. It exercises only
//! the crate's public API and adds no new dependencies.

use platform_contracts::{parse_contract_identifier, Envelope, Readiness, ReadinessStatus};

fn main() {
    // Typed payloads mirror the language-neutral schema families.
    let payload = Readiness {
        service: "billing-api".to_string(),
        status: ReadinessStatus::Ready,
        checked_at: "2026-09-30T00:00:00Z".to_string(),
        checks: None,
        tenant_id: None,
    };

    // Every contract document travels inside an envelope.
    let envelope = Envelope::new(
        "platform.readiness/0.1.0",
        time::OffsetDateTime::UNIX_EPOCH,
        payload,
    );
    let document = serde_json::to_value(&envelope).expect("envelope serializes");
    assert_eq!(document["contract"], "platform.readiness/0.1.0");

    // Contract identifiers parse into a family plus a semver triple.
    let (family, version) =
        parse_contract_identifier("platform.readiness/0.1.0").expect("valid identifier");
    assert_eq!(family, "platform.readiness");
    assert_eq!((version.major, version.minor, version.patch), (0, 1, 0));
}
