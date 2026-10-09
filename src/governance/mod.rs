//! Standalone-first governance provider contract.
//!
//! Forge owns the local provider and normalized observation model. External
//! governance systems are optional executable adapters that exchange bounded
//! JSON and never become Forge dependencies.

pub mod contract;
pub mod conversions;
pub mod defaults;
pub mod engine;
pub mod engine_tests;
pub mod model;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
