//! Foundation primitives shared by every other platform crate.
//!
//! This crate provides four families of types:
//!
//! 1. **Identifier primitives** — [`id::Id<T>`] (phantom-typed opaque IDs),
//!    [`bounded::BoundedString<N>`] (length-bounded strings), and
//!    [`correlation::CorrelationId`] (UUIDv4 with a typed wrapper).
//! 2. **Clock abstraction** — [`clock::Clock`] trait plus the
//!    [`clock::SystemClock`] and [`clock::FixedClock`] implementations so
//!    applications and tests can share a single "now" source.
//! 3. **Safe error model** — [`error::ErrorCategory`] and
//!    [`error::SafeError`]. The error model never carries credential
//!    material; secret values belong in [`redaction::Secret`].
//! 4. **Redaction** — [`redaction::Secret<T>`] wrapper whose `Debug` and
//!    `Display` implementations render `<redacted>` so secrets cannot leak
//!    through formatter-based logging or panic messages.

#![forbid(unsafe_code)]

pub mod bounded;
pub mod clock;
pub mod correlation;
pub mod error;
pub mod id;
pub mod redaction;
pub mod result;

pub use bounded::{BoundedString, BoundedStringError};
pub use clock::{Clock, FixedClock, SystemClock};
pub use correlation::CorrelationId;
pub use error::{ErrorCategory, SafeError, SafeErrorKind};
pub use id::Id;
pub use redaction::Secret;
pub use result::PlatformResult;
