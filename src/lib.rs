//! Forge Core and Registry library.
//!
//! Core owns the versioned project model, schema validation and typed
//! outcomes. Transports (CLI now; MCP/API later) render Core outcomes
//! without reinterpreting them.

pub mod agent;
pub mod core;
pub mod distribution;
pub mod docs;
pub mod doctor;
pub mod feature;
pub mod generate;
pub mod gitops;
pub mod import;
pub mod mcp;
pub mod policy;
pub mod profile;
pub mod registry;
pub mod spec;
pub mod upgrade;
