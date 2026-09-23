//! Forge Core and Registry library.
//!
//! Core owns the versioned project model, schema validation and typed
//! outcomes. Transports (CLI now; MCP/API later) render Core outcomes
//! without reinterpreting them.

pub mod agent;
pub mod analytics;
pub mod api;
pub mod checker;
pub mod component;
pub mod core;
pub mod deploy;
pub mod distribution;
pub mod docs;
pub mod doctor;
pub mod feature;
pub mod fleet;
pub mod generate;
pub mod gitops;
pub mod governance;
pub mod identity;
pub mod import;
pub mod mcp;
pub mod planner;
pub mod policy;
pub mod portal;
pub mod procedure;
pub mod profile;
pub mod provider;
pub mod readiness;
pub mod registry;
pub mod release;
pub mod spec;
pub mod ui_pattern;
pub mod upgrade;
