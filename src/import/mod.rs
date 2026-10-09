//! Conservative onboarding of existing repositories (`project-import`).
//!
//! Detection is strictly read-only: filesystem probes and argument-array
//! Git inspection never modify the target directory. A manifest file is
//! written only by [`adopt_import`], only after the proposal is accepted
//! (explicit `--accept`), and only after registry identity is pre-checked,
//! so failures leave source and conflicting metadata unchanged.
//!
//! Ambiguous repositories (monorepos, mixed frameworks) never resolve by
//! silently taking the first detector: inspection fails with
//! `error[ambiguous-import]` until the caller selects a profile or a more
//! specific path.

pub mod contract;
pub mod detect;
pub mod display;
pub mod model;
pub mod sync;

// Re-export all types
pub use contract::*;
pub use detect::*;
pub use model::*;
pub use sync::*;
