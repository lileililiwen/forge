//! Versioned profile descriptors and the stack compatibility contract.
//!
//! Core owns the descriptors; transports render Core outcomes without
//! reinterpreting them. Descriptors are compiled-in, versioned metadata —
//! they establish what each stack supports, not working templates.
//! Generation validates each supported stack separately with its native
//! toolchain before any profile support is advertised as verified.
//!
//! MVP IDs: `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`,
//! `python-service`, plus `react-web` (promoted from the v0.2 catalog with
//! the same descriptor contract and a tested native scaffold). Future
//! specialist candidates are reserved as [`ProfileSupportStatus::Planned`]
//! descriptors so the roadmap stays discoverable without advertising
//! unsupported generation: a planned id is not selectable through
//! resolution, preflight or generation until it is promoted to
//! [`ProfileSupportStatus::Supported`].
//!
//! Versions are explicit; there is no implicit `latest`.

pub mod capabilities;
pub mod catalog;
pub mod model;
pub mod resolve;

// Re-export all types
pub use catalog::*;
pub use model::*;
pub use resolve::*;
