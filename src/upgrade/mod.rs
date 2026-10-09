//! Deterministic project and fleet upgrades (`project-upgrade-orchestration`).
//!
//! Core owns upgrade planning and application; transports render Core
//! outcomes without reinterpreting them.
//!
//! A [`UpgradePlan`] is pinned and read-only: exact old/new versions,
//! affected assets, validators and recovery implications, computed before
//! any mutation. [`apply_upgrade`] validates compatibility and ownership
//! preconditions before writing; a drifted ownership receipt means
//! user-owned edits are present, so the project is left unchanged and a
//! semantic-conflict handoff (naming the owned file and the suggested
//! `forge spec generate` follow-up) is emitted instead of overwriting them.
//! Already-satisfied upgrades are a no-op: no file, manifest or registry
//! write occurs.
//!
//! Fleet runs ([`run_fleet`]) capture an explicit registry selection up
//! front, journal each project independently (`upgrade` operation rows with
//! `done`/`failed`/`blocked`/`skipped` states) and report distinct
//! per-project statuses without marking the fleet wholly successful when
//! any project fails or blocks. Retries re-plan from current manifest
//! state, so completed steps are not repeated and changed preconditions
//! cause fresh conflict reporting.
//!
//! Recovery model: file recovery (manifest bytes plus receipts) is
//! reversible; database schema migrations are NOT. The `postgres` feature
//! declares strategy `manifest-repin+manual-schema-review` and its plan
//! steps are marked irreversible: filesystem rollback does not reverse
//! applied database migrations, so a backup plus manual data recovery is
//! required.

pub mod contract;
pub mod engine;
pub mod model;
pub mod upgrade_tests;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
