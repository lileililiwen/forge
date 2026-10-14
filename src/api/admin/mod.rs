//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

pub mod agent_identity;
pub mod assurance;
pub mod creation;
pub mod deploy;
pub mod gate;
pub mod gateway;
pub mod history;
pub mod identity;
pub mod interest;
pub mod model;
pub mod publish;
pub mod release;
pub mod routes;
pub mod shipping_reads;

// Re-export all types
pub(super) use deploy::*;
pub(super) use gateway::*;
pub use routes::*;

use super::Route;

/// Match the beside-the-table read-only admin routes (history,
/// agent/identity, creation catalog, shipping reads, assurance, gate)
/// plus the global identity write (change-password) before the main
/// table runs. One call site keeps `router.rs` under the source-file-
/// size cap; the per-family matchers own their shapes and shadow no
/// table arm. The `identity` literal in `route_identity` precedes the
/// project-table `{id}` arms so it never reads as a project id.
pub(in crate::api) fn route_beside(method: &str, segments: &[&str]) -> Option<Route> {
    history::route_history(method, segments)
        .or_else(|| agent_identity::route_agent_identity(method, segments))
        .or_else(|| identity::route_identity(method, segments))
        .or_else(|| creation::route_creation(method, segments))
        .or_else(|| shipping_reads::route_shipping_reads(method, segments))
        .or_else(|| assurance::route_assurance(method, segments))
        .or_else(|| gate::route_gate(method, segments))
        .or_else(|| interest::route_interest(method, segments))
}
