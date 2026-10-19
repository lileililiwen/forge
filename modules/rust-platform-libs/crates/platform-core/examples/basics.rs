//! Minimal `platform-core` usage: bounded IDs, a clock, and a safe error.
//!
//! This example is the compile-checked companion to the `platform-core`
//! section of `docs/usage.md`. It exercises only the crate's public API
//! and adds no new dependencies.

use platform_core::{
    BoundedString, Clock, CorrelationId, ErrorCategory, FixedClock, Id, SafeError,
};

/// Phantom tag giving the opaque ID its type.
struct UserTag;

fn main() {
    // Length-bounded strings reject over-long input at construction.
    let name = BoundedString::<64>::try_new("alice").expect("short name fits");
    assert_eq!(name.as_str(), "alice");

    // Opaque, phantom-typed identifiers.
    let user_id = Id::<UserTag>::new("usr_alice");
    assert_eq!(user_id.as_str(), "usr_alice");

    // One shared "now" source; tests pin it with `FixedClock`.
    let clock = FixedClock::at_epoch();
    let _now = clock.now();

    // Correlation IDs travel with requests across crates.
    let correlation_id = CorrelationId::new();

    // Safe errors carry a stable category and never embed secrets.
    let err = SafeError::new(ErrorCategory::Unavailable, "upstream timed out")
        .with_correlation(correlation_id);
    assert_eq!(err.category(), ErrorCategory::Unavailable);
    println!("{err}");
}
