//! Jenkins/Mac publish orchestration (`jenkins-publish-integration`).
//!
//! Core owns the *generic* publish contract: actions, requests,
//! reports, stage outcomes, journal labels, dry-run gating. The
//! *mechanism* (today: Jenkins over SSH) lives behind the
//! [`PublishAdapter`] trait so the core can stay stable while
//! alternative adapters (raw SSH, GitHub Actions, local-compose) are
//! added without touching it.
//!
//! ## Architecture
//!
//! Three layers, each replaceable independently:
//!
//! 1. **Core** — [`PublishAction`], [`PublishRequest`],
//!    [`PublishReport`], [`StageOutcome`]. Knows about stages and
//!    outcomes, not about SSH or Jenkins.
//! 2. **Transport** — [`SshTransport`] is the only I/O boundary.
//!    Subprocesses go through it; tests inject a recording transport.
//! 3. **Adapter** — implements [`PublishAdapter`] for one mechanism.
//!    [`jenkins::JenkinsAdapter`] is the shipped implementation;
//!    other adapters (raw SSH, GitHub Actions) follow the same trait.
//!
//! ## Stages
//!
//! 1. **Sync** — make the project's source tree available to the
//!    target host (adapter-specific; SSH+rsh for Jenkins, git push
//!    for Actions).
//! 2. **Prepare** — ensure the project has ports, env files, and
//!    any other host-level prerequisites.
//! 3. **Deploy** — trigger the production deploy job.
//!
//! ## Dry-run mode
//!
//! Every stage is gated on the `dry_run` flag. A dry run records the
//! would-be command in the stage outcome, marks the stage as
//! `dry-run`, and exits without invoking any subprocess.
//!
//! ## Risk model
//!
//! Publishing to a remote host is irreversible from Forge's
//! perspective. The contract refuses to deploy without explicit
//! `--confirm` (the CLI's `all` action implies confirmation),
//! preserves the prior partial state on failure, and never claims
//! success on an unreachable target.

pub mod caddy;
pub mod contract;
pub mod db_overlay;
pub mod fleet;
pub mod github;
pub mod inventory;
pub mod jenkins;
pub mod model;
pub mod pipeline;
pub mod port_allocator;
pub mod providers;
pub mod recording_transport;
pub mod remote_compose;
pub mod subprocess_transport;

// Re-export all types
pub use contract::*;
pub use model::*;
pub use pipeline::*;
