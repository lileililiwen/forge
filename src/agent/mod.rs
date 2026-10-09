//! Managed agent session contract (`agent-runtime-workflows`).
//!
//! Core owns the versioned session model and the provider-neutral
//! transition rules; transports (CLI now; MCP/API later) render Core
//! outcomes without reinterpreting them.
//!
//! The contract is intentionally honest about provider support:
//! the bundled OpenCode/Codex adapters' `start`/`new_session`
//! always succeed when the adapter binary is present on PATH, and
//! `resume`/`restart` follow the recorded transition log. On the
//! bundled adapters `pause` and `takeover` remain explicitly
//! `unsupported` because the bundled surface does not expose those
//! primitives. The `ariadex` provider instead delegates every
//! transition to the real supervised runtime (`supervised-agent-
//! adapters`): ordered binary resolution (`FORGE_ARIADEX_BIN`
//! first, then the PATH name), argument-array invocation with a
//! bounded wait, and a state claim taken only from what
//! `ariadex status --json` actually reported — an unknown or
//! unlive runtime maps to `disconnected`, never to `active`,
//! and `takeover` records attach guidance instead of hijacking
//! the operator's terminal.
//!
//! Session storage lives under the project root so each project
//! owns its own sessions and the registry's operation journal
//! records the originating operation:
//!
//! ```text
//! .forge/agents/
//!   <session-id>/
//!     session.json   AgentSession with provider, spec, transitions
//!     transitions.log  append-only transition evidence
//! ```
//!
//! The contract version is recorded in every session and the
//! recorded transitions, so a future schema bump can refuse
//! incompatible readers instead of silently reinterpreting
//! history. Spec execution is routed to the recorded spec id
//! (R1 success scenario); a missing spec surfaces as
//! `error[spec-invalid]` without changing the project.

#[cfg(test)]
pub mod agent_tests;
pub mod constants;
pub mod model;
pub mod sessions;
pub mod transitions;

// Re-export all types
pub use constants::*;
pub use model::*;
pub use sessions::*;
pub use transitions::*;
