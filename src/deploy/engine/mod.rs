//! Deployment preparation, plan rendering, apply and
//! observation (`adapter-deployment`).
//!
//! This is the Core side of the deploy engine. The CLI and
//! MCP transports build a [`DeployRequest`], call
//! [`prepare_deploy`], [`apply_deploy`] or [`observe_deploy`],
//! and render the returned [`DeployReport`]. No transport
//! reinterprets the typed outcomes; the contract owns the
//! labels and the per-stage semantics.

pub mod model;
pub mod stages;
pub mod stages_tests;

// Re-export all types
pub use stages::*;
