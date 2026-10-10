//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

pub mod agent_identity;
pub mod assurance;
pub mod creation;
pub mod deploy;
pub mod gateway;
pub mod history;
pub mod model;
pub mod publish;
pub mod release;
pub mod routes;

// Re-export all types
pub(super) use deploy::*;
pub(super) use gateway::*;
pub use routes::*;

use super::Route;

/// Match the beside-the-table read-only admin routes (history,
/// agent/identity, creation catalog, assurance) before the main table runs.
/// One call site keeps `router.rs` under the source-file-size cap;
/// the per-family matchers own their shapes and shadow no table arm.
pub(in crate::api) fn route_beside(method: &str, segments: &[&str]) -> Option<Route> {
    history::route_history(method, segments)
        .or_else(|| agent_identity::route_agent_identity(method, segments))
        .or_else(|| creation::route_creation(method, segments))
        .or_else(|| assurance::route_assurance(method, segments))
}
