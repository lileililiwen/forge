//! Deterministic new-project assembly (`deterministic-project-generation`).
//!
//! Explicit flags and interactive answers normalize into one
//! [`CreationRequest`]. Pinned profile assets ([`GENERATOR_VERSION`]) render
//! into a staging directory, paths are validated to stay inside the
//! destination, then output is promoted into an empty destination and
//! registered. Rendering never needs the stack toolchain; native build/test
//! evidence is reported separately by [`verify_native`] and never claimed
//! from rendering alone.
//!
//! Generated projects own ordinary source and depend on no Forge runtime.
//! Feature flags are recorded (pinned `0.1.0`) after compatibility checks;
//! installing v0.2 feature behavior stays out of scope.
//!
//! Documented identity fields: registry `path` (absolute destination) and
//! `observed_at` timestamps may differ between otherwise equivalent
//! creations. Rendered file bytes are otherwise byte-identical for identical
//! requests.

pub mod constants;
pub mod engine;
pub mod generate_tests;
pub mod model;
pub mod request;
pub mod workspace;

// Re-export all types
pub use constants::*;
pub use engine::*;
pub use model::*;
pub use request::*;
