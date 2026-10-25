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
//! - Forge composes that identity only for a response that published.
//!   A response that published nothing carries no identity, so no row
//!   names a container for a deployment that never happened.

pub mod contract;
pub mod engine;
pub mod engine_tests;
pub mod model;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;

/// The provider terminal statuses that say the publish completed.
///
/// The lanes already read two words as a completed run: the publish
/// contract's `done` and the delivery lane's `succeeded`. Both mean
/// the provider published.
pub const PUBLISH_COMPLETED_STATUSES: [&str; 2] = ["done", "succeeded"];

/// The container identity a publish response proves.
///
/// The provider's own identity is the truth and is always kept. A
/// legacy provider that published but reported no identity gets the
/// canonical `forge-<project>-<sha12>` identity composed from the
/// recorded revision, so it is not silently absent from observability.
/// A response that published nothing carries no identity at all: Forge
/// does not invent a container for a deployment that never happened.
pub fn proven_container_identity(
    response: &PublishProviderResponse,
    project_id: &str,
    recorded_revision: &str,
) -> Option<String> {
    if let Some(identity) = response.container_identity.as_deref() {
        return Some(identity.to_string());
    }
    if !PUBLISH_COMPLETED_STATUSES.contains(&response.status.as_str()) {
        return None;
    }
    Some(compose_project_name(project_id, recorded_revision))
}
