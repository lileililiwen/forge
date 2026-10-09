//! Release preparation, plan rendering and apply logic.
//!
//! This is the Core side of the release engine. The CLI and
//! MCP transports build a [`ReleaseRequest`], call
//! [`prepare_release`] or [`apply_release`], and render the
//! returned [`ReleaseReport`]. No transport reinterprets the
//! typed outcomes; the contract owns the labels and the
//! per-stage semantics.

pub mod apply;
pub mod engine_tests;
pub mod model;
pub mod operations;
pub mod prepare;
pub mod stages;

// Re-export all types
pub use apply::*;
pub use model::*;
pub use operations::*;
pub use prepare::*;
