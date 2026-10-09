//! Bounded GitHub metadata adapter invocation
//! (`github-project-metadata-adapter`).
//!
//! The adapter is an external executable that speaks the
//! `forge-github-metadata/0.1.0` request/response contract. Forge never
//! embeds the GitHub SDK and never makes an HTTP request itself: it
//! spawns the adapter with `Command::new` plus an argument array, with
//! `LC_ALL=C` and a bounded per-run timeout, and parses the JSON
//! response.
//!
//! ## Configuration
//!
//! The adapter binary is resolved in this order:
//!
//! 1. `FORGE_GITHUB_BIN` (always wins when set and the file is
//!    executable).
//! 2. `forge-github-metadata-adapter` on `PATH`.
//!
//! The token is read from `FORGE_GITHUB_TOKEN` once and is **never**
//! passed on the command line, never written to a journal row and
//! never echoed on stdout or stderr. The adapter reads it through the
//! inherited environment variable.
//!
//! ## States
//!
//! The closed [`GithubState`] vocabulary distinguishes success,
//! transient rate limits, credential problems, missing repositories,
//! forbidden access, network unavailability, staleness and partial
//! responses. A partial or stale observation is **not** a complete
//! record; the catalog records the state and the operator decides
//! whether to act on it.
//!
//! ## Mutation
//!
//! [`MutationMode::PullRequest`] is the default and the only mode that
//! is accepted without a separate confirmation. [`MutationMode::Direct`]
//! is refused unless the caller passes an explicit confirmation token
//! that the adapter echoes back, so a misconfigured caller cannot push
//! silently. The package never changes repository settings implicitly
//! and never opens issues, comments, labels or pull requests for
//! side-effect data.

pub mod limits;
pub mod model;
pub mod state_display;
pub mod support;

// Re-export all types
pub use limits::*;
pub use model::*;
