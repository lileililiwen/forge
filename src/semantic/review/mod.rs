//! Suggest / approve / reject / supersede / list / read
//! (`project-semantic-description-review`).
//!
//! The package owns the operations that move a proposal through
//! the closed state machine. The operations are deliberately
//! conservative:
//!
//! - `suggest` only stores `Suggested` proposals. A later suggestion
//!   for the same kind and a different evidence revision
//!   supersedes the prior proposal rather than merging into it.
//! - `approve` and `reject` only move a `Suggested` proposal to
//!   `Approved` or `Rejected` respectively. The operation never
//!   applies the suggestion to a project file or a provider; the
//!   remediation and adapter packages own the actual write.
//! - `supersede` is the implicit transition that a fresh
//!   `suggest` triggers on the prior open proposal of the same
//!   kind.
//! - `read` and `list` are read-only and never mutate state.
//!
//! The provider model is intentionally closed. An unavailable
//! provider never produces a placeholder proposal; the
//! [`SuggestOutcome`] reports an `unavailable` state so a
//! transport can surface it without inventing a record.

pub mod decide;
pub mod fields;
mod ids;
pub mod model;
pub mod suggest;

// Re-export all types
pub use decide::*;
pub use fields::*;
pub use model::*;
pub use suggest::*;
