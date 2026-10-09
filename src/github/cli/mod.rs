//! Bounded `gh` CLI adapter for local repository workflows
//! (`github-cli-project-workflows`).
//!
//! Forge never embeds the GitHub SDK and never makes an HTTP request
//! itself: it spawns the installed `gh` CLI with `Command::new` plus a
//! fixed argument array, with `LC_ALL=C` and the shared bounded
//! per-run timeout from [`crate::process`], and parses the outcome.
//!
//! ## Configuration
//!
//! The `gh` binary is resolved in this order:
//!
//! 1. `FORGE_GH_BIN` (always wins when set and the file is
//!    executable).
//! 2. `gh` on `PATH`.
//!
//! The user's `gh` auth context is read through `gh auth status`.
//! Forge never spawns `gh auth token`, never reads the credential
//! store directly, and never inherits the credential through any
//! environment variable of its own. The token lives in the user's
//! `gh` credential store; Forge only learns "authenticated" / "not
//! authenticated" through `gh auth status`.
//!
//! ## States
//!
//! The closed [`GhOutcome`] vocabulary distinguishes success,
//! missing CLI, missing authentication, scope problems, missing
//! repositories, conflicting state, rate limits, timeouts, generic
//! unavailability and generic failure. A `failed` or `unavailable`
//! outcome is **not** a complete record; the journal records the
//! state and the operator decides whether to retry.
//!
//! ## Writes
//!
//! Repository creation, source push and pull-request creation all
//! require an explicit `--confirm` so a misconfigured caller cannot
//! publish silently. Public visibility requires the explicit pair
//! `--visibility public --confirm-public`. The package never opens
//! browser flows, never merges PRs and never creates releases.

pub mod commands;
pub mod limits;
pub mod model;
pub mod operation_display;
pub mod outcome_display;

// Re-export all types
pub use commands::*;
pub use limits::*;
pub use model::*;
