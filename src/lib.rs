//! Forge Core and Registry library.
//!
//! Core owns the versioned project model, schema validation and typed
//! outcomes. Transports (CLI now; MCP/API later) render Core outcomes
//! without reinterpreting them.

pub mod core;
pub mod registry;
