//! Forge-owned remote-compose publish adapter
//! (`decoupled-remote-publish`).
//!
//! This adapter publishes a project to a generic Docker host that
//! carries **no deployment script tree**: the target needs Docker and
//! a file tree (`projects/<id>/`, `runtime/<id>/`, `platform/`) and
//! nothing else. Every stage is expressed as plain `ssh`, `rsync`,
//! `scp`, `mkdir`/`mv` and `docker compose` argument arrays issued
//! from Linux.
//!
//! | Stage | What Forge does |
//! |---|---|
//! | `Sync` | `ssh <t> mkdir -p` then `rsync -az` of the source tree |
//! | `Prepare` | read `RUNTIME_ROOT/port-registry.json` over SSH, allocate the 20-port block in Rust, render `ports.compose.yml` (and the shared-DB overlay) locally, ship both |
//! | `Db` | `docker compose -f shared-infra/compose.yml up -d production-postgres` plus a `production-db-network` check |
//! | `Deploy` | `docker compose -p forge-<id> … up -d --build`, then ship `platform/Caddyfile` + `site/index.html` and recreate the router |
//!
//! ## Port allocation and overlays are Forge-owned
//!
//! [`super::port_allocator`], [`super::db_overlay`] and [`super::caddy`]
//! carry the semantics the target scripts used to own. Forge collects
//! the inputs it needs from the target (`port-registry.json`, the
//! project Compose config, the ports Docker already binds) during
//! [`PublishAdapter::materialize`], computes every document locally,
//! and ships only finished files.
//!
//! ## Secrets stay on the target
//!
//! The adapter never reads `SECRETS_ROOT/<project>/.env` or
//! `.shared-db.env`. It probes only whether they exist (`ssh <t> test
//! -f <path>`) and passes them to Compose as repeated `--env-file`
//! arguments, which is the shell-free equivalent of the legacy
//! merge into `runtime/<project>/production.env`. Every captured
//! string passes [`crate::policy::redact_credentials`] before it can
//! reach a journal row or a rendered document.
//!
//! ## Migration
//!
//! [`super::jenkins::JenkinsAdapter`] (`ssh bash scripts/*.sh`) stays
//! compiled and selectable through [`ADAPTER_ENV`] so a real-deploy
//! failure rolls back with a flag, not a rebuild.

pub mod adapter;
pub mod adapter_tests;
pub mod constants;
pub mod model;
pub mod stages;

// Re-export all types
pub use constants::*;
pub use model::*;
pub use stages::*;
