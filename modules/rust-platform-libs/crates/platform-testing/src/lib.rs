//! Deterministic fakes, builders, and assertions for testing platform
//! code.
//!
//! This crate is the only place in the workspace that is allowed to
//! depend on every other platform crate. Production crates
//! **MUST NOT** depend on it; the architecture test
//! (`platform-arch-tests`) enforces that direction.
//!
//! The crate provides:
//!
//! * [`clock::FakeClock`] — a re-export of the core `FixedClock` with
//!   a `FakeClock` alias that makes test intent explicit.
//! * [`subject::TestSubject`] — a builder for `platform_identity::Subject`
//!   values that pick safe defaults.
//! * [`reasons::assert_redacted!`] — a debug assertion that a string
//!   does not contain any of the registered secret substrings.
//! * [`fixtures`] — re-exports of canonical fixture documents from
//!   the `platform-contracts` repository, loadable in tests.

#![forbid(unsafe_code)]

pub mod clock;
pub mod evaluator;
pub mod fixtures;
pub mod reasons;
pub mod subject;

pub use clock::FakeClock;
pub use evaluator::{ScriptedEvaluatorError, ScriptedPermissionEvaluator};
pub use reasons::assert_redacted;
pub use subject::TestSubject;
