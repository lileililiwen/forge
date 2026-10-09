//! Adapter-based deployment and observed runtime state (`adapter-deployment`).
//!
//! Core owns the typed deployment contract. v0.1.0 supports:
//!
//! - [`DeployConfig`] parses the manifest's `deployment` block,
//!   refuses empty or duplicate targets, accepts only the
//!   supported adapter kinds (`local`, `docker-compose`, `jenkins`,
//!   `mac-runtime`; `ssh` is planned) and confines artifact paths to the
//!   project directory.
//! - [`prepare_deploy`] captures the named target, the
//!   artifact identity (path, content hash), the working-tree
//!   revision and the configured health check into a
//!   reviewable [`DeployPlan`]. No side effect runs from
//!   `prepare`.
//! - [`apply_deploy`] refuses without `--confirm`; walks the
//!   per-target adapter invocation through the configured
//!   `FORGE_DEPLOYER_BIN` binary, captures the timestamped
//!   health observation and persists a [`DeployState`]
//!   under `.forge/deploy/<project-id>/<deploy-id>/state.json`.
//! - [`observe_deploy`] re-runs the health check after a
//!   successful apply (or on demand) and updates the same
//!   state: a passing observation records `running`; a
//!   failing observation records `failed`; an unreachable
//!   target leaves the last successful observation as
//!   `last_observed_running` and the current state as
//!   `unknown` (R2 boundary: disconnected is unknown, not
//!   offline proof).
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §30, §34, §43 require
//! simple deployment targets and observed runtime state. The
//! contract is independent of any real Docker daemon, SSH
//! endpoint or external coordinator: a missing adapter
//! binary, an unsupported target, a stale plan or a target
//! that refuses the connection surfaces as a typed
//! `error[...]` response before any health observation is
//! recorded.
//!
//! ## Persistence
//!
//! [`DeployState`] lives under
//! `.forge/deploy/<project-id>/<deploy-id>/state.json` so a
//! retry sees exactly which target already delivered and
//! which needs another attempt. The state is local
//! evidence, not a record of authority: a successful run
//! overwrites the prior entry, a failed run leaves the last
//! good observation untouched. The Core registry's
//! `operations` table receives one `deploy` row per
//! prepare/apply/observe with a `done`/`partial`/`blocked`
//! summary.
//!
//! ## Risk model
//!
//! A remote write is irreversible. The contract refuses to
//! apply a deploy without an explicit `--confirm`; the
//! contract refuses to overwrite a successful health
//! observation with a stale one. Credentials embedded in
//! evidence are redacted through
//! [`crate::policy::redact_credentials`].
//!
//! Real provider integration is out of scope for v0.1.0:
//! the `local` and `docker-compose` adapters reuse the same
//! `FORGE_DEPLOYER_BIN` environment-variable pattern the
//! docs and policy contracts use, so a fixture binary
//! stands in for a real `docker compose` or `scp` round
//! trip.

pub mod contract;
pub mod engine;
pub mod model;
pub mod prepare;

// Re-export all types
pub use contract::*;
pub use model::*;
pub use prepare::*;
