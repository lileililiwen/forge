//! Existing content and analytics planes with project metrics
//! (`external-planes-analytics`).
//!
//! Core owns the typed contract for the manifest's
//! `analytics:` block, the external provider health
//! observations and the timestamped project metrics
//! aggregation. The architecture calls for keeping external
//! systems' identities — Forge records references and
//! health, not a duplicate CMS or analytics backend.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §32, §33 require
//! the existing content and analytics providers to retain
//! their identities while Forge aggregates the
//! project-level metrics the brief calls for. The contract
//! is provider-agnostic: the manifest declares the
//! reference and the adapter binary; the contract reads
//! the reference, runs the adapter with argument arrays
//! (never shell) and surfaces a typed observation. A
//! missing adapter, a refused provider, an inconsistent
//! mapping or a disabled integration all surface as
//! typed `error[...]` responses before any state is
//! mutated.
//!
//! ## Boundary
//!
//! - **Existing systems keep their identity.** The manifest
//!   carries the external project reference (e.g.
//!   `owner/repo` for GitHub or a content slug for the
//!   unified content plane). Forge never rewrites the
//!   upstream system and never claims ownership of its
//!   data — the contract reports the reference and the
//!   health, nothing more.
//! - **Disabled integrations do not contact the provider.**
//!   The `enabled: false` master switch and per-entry
//!   `enabled: false` flag both surface a `disabled`
//!   observation without invoking the adapter (R1
//!   boundary scenario).
//! - **Metrics are timestamped, not summed across unknown
//!   windows.** Each observation records its source, the
//!   timestamp it was captured at, and the explicit
//!   observation window. A metric whose provider did not
//!   return data is reported as `unavailable`, not
//!   silently zeroed, and never summed with a metric from
//!   a different window (R2 failure and boundary
//!   scenarios).
//! - **Adapters are external binaries invoked with
//!   argument arrays.** The default is
//!   `forge-analytics-adapter`, overridable through
//!   `FORGE_ANALYTICS_BIN`. The process is spawned with
//!   `Command::new` plus arguments, with a bounded
//!   per-run timeout, so an unresponsive tool cannot hang
//!   the registry. A missing binary, non-zero exit,
//!   timeout, contract mismatch or unparseable output
//!   surfaces as an `unavailable` observation, never as
//!   `available`.
//!
//! ## Persistence
//!
//! No new persistent state is added in v0.1: the analytics
//! block lives in the manifest, the health observations
//! and the metrics aggregation are computed on demand
//! from the local registry, doctor findings, deploy
//! states, identity sessions and adapter invocations. The
//! Core registry's `operations` table receives one
//! `analytics` row per `inspect` / `metrics` call with a
//! `done` / `rejected` verdict and the project id (no
//! synthetic project is invented; analytics stays
//! project-scoped).
//!
//! ## Risk model
//!
//! A real provider round trip is out of scope for the
//! local sandbox: the contract is validated through
//! `FORGE_ANALYTICS_BIN` fixture shell scripts that
//! stand in for the real `github-analytics` /
//! `unified-content` adapters. The credential redaction
//! rule set is the same `policy::redact_credentials`
//! consumed by every other adapter, so a leaked secret
//! in adapter output is redacted on stdout and in the
//! journal. A real provider round trip is a downstream
//! integration step.

pub mod constants;
pub mod model;
pub mod projections;
pub mod projections_tests;

// Re-export all types
pub use constants::*;
pub use model::*;
pub use projections::*;
