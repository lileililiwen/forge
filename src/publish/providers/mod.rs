//! External Forge publish-provider contract.
//!
//! Owns the revision-bound phase evidence and the runtime identity
//! the contract guarantees:
//!
//! - Every request carries a full 40-character hex Git revision.
//! - Terminal responses may carry additive `revision`,
//!   `build_status`, `run_status`, and `container_identity` fields
//!   so `forge deploy status` can answer "what revision is running?"
//!   without contacting the provider again. Missing fields render
//!   as `unknown` in the projection — a response without phase
//!   evidence is not a verified success.
//! - `forge-<project>-<sha12>` is the canonical Compose project /
//!   container identity, where `<sha12>` is the first 12 hex
//!   characters of the committed revision. Compose files that
//!   override `container_name` to a value without the SHA fail the
//!   run phase before it reports success.

pub mod contract;
pub mod engine;
pub mod engine_tests;
pub mod model;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
