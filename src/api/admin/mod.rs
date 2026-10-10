//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

pub mod agent_identity;
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
