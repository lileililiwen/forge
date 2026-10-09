//! Controlled provider integration evidence
//! (`provider-integration-evidence`).
//!
//! The adapter boundaries (`policy`, `identity`, `analytics`, `deploy`,
//! `release`) already model `unavailable` / `disabled` / `ambiguous`
//! outcomes through contract fixtures. Fixtures alone cannot prove a real
//! DriftWatch binary, OIDC issuer, analytics source or deployment target
//! works in a live environment, and a fixture success must never be
//! reported as verified provider support.
//!
//! This module owns the opt-in evidence harness that closes that gap:
//!
//! - [`provider_ids`] lists the six evidence providers in stable order.
//! - [`matrix`] reports every provider as `not-run` unless the caller
//!   opts in with `--live`; a `not-run` row is never `supported`.
//! - [`run_controlled`] drives one controlled round trip against an
//!   explicit fixture binary (`sandbox: fixture`, supplemental) or a real
//!   sandbox binary (`sandbox: live`, verifiable) and records provider,
//!   project, revision, timestamp and the redacted receipt as
//!   [`EvidenceProvenance`].
//! - Binary probes reuse each adapter's real argument protocol with a
//!   bounded wait, always with `--dry-run` where the adapter supports it,
//!   so a probe never performs a real remote write.
//! - The identity probe runs the real in-memory OIDC lifecycle
//!   (challenge → callback → claims → mint → validate → terminate) through
//!   `crate::identity`; a real issuer round trip remains a downstream step.
//! - Every evidence string passes through [`redact_provider_evidence`],
//!   which delegates to `policy::redact_credentials`, so the evidence
//!   contract shares one definition of "secret" with every other adapter.
//! - Probes run in disposable temp dirs (or a caller-named project dir for
//!   targeted live runs) and [`teardown_probe`] removes what the probe
//!   created; the row reports `teardown: true` only when nothing remains.
//!
//! Secrets reach Forge only through the runner's environment (binary paths
//! and fixture files); manifests and the repository never carry provider
//! secrets, and the journal records only redacted receipts.

pub mod contract;
pub mod matrix;
pub mod model;
pub mod probes;
pub mod provider_tests;
pub mod runner;

// Re-export all types
pub use contract::*;
pub use matrix::*;
pub use model::*;
pub use probes::*;
pub use runner::*;
