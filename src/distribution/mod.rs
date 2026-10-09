//! Canonical primary and one-way mirror distribution
//! (`repository-distribution`).
//!
//! Core owns the typed distribution contract. v0.1.0 supports a
//! single configured primary and provider-qualified mirrors with
//! one-way semantics: a successful primary push is recorded
//! independently from each mirror push, a mirror push that fails
//! is reported without misreporting the primary state, and a
//! retry executes only the outstanding work. GitLab/Codeberg
//! providers are declared but refused at plan time so the
//! registry does not silently grow past what is verified.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §27, §32, §34, §43 require
//! a canonical primary and one-way mirrors with per-remote
//! outcomes and credential redaction. The distribution contract
//! stays independent of any real provider integration: an
//! unauthenticated mirror attempt, a divergent protected
//! history, or a disabled mirror all surface as typed
//! `error[...]` responses before any file or registry state is
//! mutated, matching the existing upgrade / push
//! confirm-required boundary.
//!
//! ## Persistence
//!
//! [`MirrorState`] is stored under
//! `.forge/distribution/<project-id>/state.json` so a retry can
//! see which mirrors received which refs. The state is local
//! evidence, not a record of authority: a successful retry
//! overwrites the previous entry. The Core registry's
//! `operations` table receives one `mirror` row per run, with
//! a `done`/`partial`/`failed` summary that lists the per-remote
//! states (the per-remote evidence is redacted so credentials do
//! not leak through the journal).
//!
//! ## Risk model
//!
//! The primary and mirror refs are never pushed without an
//! explicit `confirm: true` (CLI `--confirm` or MCP `confirm`).
//! The contract refuses a push to a remote whose history has
//! diverged (R1 failure scenario) and a push that requires
//! `--force` (out of scope for one-way distribution).
//! Credentials are never read from the manifest; the contract
//! surfaces authentication failures with the embedded secret
//! redacted by [`crate::policy::redact_credentials`].

pub mod contract;
pub mod engine;
pub mod engine_tests;
pub mod model;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
