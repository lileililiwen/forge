//! `forge` CLI transport: argument parsing and output rendering only.
//!
//! All project rules live in Core; this layer maps typed outcomes to
//! human text or JSON plus exit codes, independently of any GUI, AI or
//! network service.

pub mod agent;
pub mod analytics;
pub mod api;
pub mod cap;
pub mod catalog;
pub mod commands;
pub mod commands_ops;
pub mod commands_portfolio;
pub mod commands_services;
pub mod component;
pub mod constants;
pub mod delivery;
pub mod feature;
pub mod fleet;
pub mod fleet_exec;
pub mod functions_10;
pub mod functions_13;
pub mod functions_9;
pub mod gaps;
pub mod github;
pub mod gitops;
pub mod governance;
pub mod identity;
pub mod intent;
pub mod inventory;
pub mod plugins;
pub mod procedure;
pub mod project;
pub mod projects;
pub mod provider;
pub mod publish;
pub mod readiness;
pub mod release;
pub mod semantic;
pub mod setup;
pub mod ui_pattern;

// `cli` holds the per-surface command handlers; the binary root
// (`src/main.rs`) keeps the `Cli`/`Commands` shape plus dispatch.
