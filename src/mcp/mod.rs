//! Mature MCP surface (`mature-mcp-surface`).
//!
//! One stdio JSON-RPC 2.0 server that exposes the stable mature
//! Core operations through a single tool list. Core owns every
//! effect: the MCP transport only validates input, dispatches
//! through the same Core contracts the CLI uses, and renders
//! typed outcomes back to the caller. No business rule is
//! duplicated in the transport.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §20, §34, §42 require
//! one MCP server over mature Core operations. v0.4 exposes
//! only the operations that have already passed local
//! verification in earlier changes; an operation that is not
//! in [`tool_registry`] is not advertised (R1 boundary
//! scenario). The first MCP version is stdio; HTTP and the
//! portal are separate changes.
//!
//! ## Tool registry
//!
//! Every entry records the tool id, a human description, the
//! [`McpToolKind`] classification, and the structured input
//! shape Core expects. The classification drives both
//! authorization and the rendering of mutating side effects:
//!
//! - `ReadOnly` — no side effects, no project mutation
//!   (for example `list_projects`, `inspect_project`,
//!   `list_profiles`, `list_features`, `run_doctor`).
//! - `Mutating` — writes files, manifest, receipts, registry
//!   or sessions inside the project scope (for example
//!   `create_project`, `import_project`, `add_feature`,
//!   `remove_feature`, `upgrade_feature`, `generate_spec`,
//!   `run_agent`, `run_tests`, `commit`).
//! - `ExternalWrite` — issues a network side effect (for
//!   example `push`). Requires an explicit `confirm: true`
//!   in the request body so an implicit remote write is
//!   never accepted.
//!
//! `deploy` and `publish` are intentionally absent. Their
//! Core operations are not implemented yet, so the
//! registry does not advertise them (R2 boundary scenario).
//!
//! ## Risk model
//!
//! Model-issued requests are untrusted structured inputs.
//! The transport validates every field by name and type
//! before any Core call; a missing required field, an
//! unknown tool, an unauthorized project path, or a name
//! that contains shell metacharacters returns a structured
//! error before Core mutates state. Diagnostic logging is
//! restricted to stderr; the JSON-RPC envelope over stdout
//! is the only response surface. The session is bounded by
//! the host process: one request per line, one response per
//! line, no remote lifecycle.

pub mod constants;
pub mod mcp_tests;
pub mod methods;
pub mod model;
pub mod params;
pub mod tools;
pub mod transport;

// Re-export all types
pub use constants::*;
pub use methods::*;
pub use model::*;
pub use tools::*;
pub use transport::*;
