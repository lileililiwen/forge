//! Delegated policy execution and quality result isolation
//! (`quality-policy-integration`).
//!
//! Core owns the typed policy adapter contract and the credential-redaction
//! pipeline. Transports (CLI now; MCP/API later) invoke [`run_driftwatch`]
//! through the same code path the registry uses for observations, so a
//! project-scoped execution produces an observation that names only its own
//! project, evidence is redacted before any caller sees it, and stale
//! observations are surfaced as such.
//!
//! The contract is versioned through [`POLICY_CONTRACT_VERSION`]; the
//! adapter speaks JSON on stdout and never through a shell, so a credential
//! in an evidence string cannot be expanded as a command. External output
//! and tool versions may drift; this module therefore reports
//! [`PolicyOutcome::Unavailable`] when the binary is missing, the
//! invocation times out or the output is not parseable JSON, and it never
//! maps an absent policy run to a `PASS`.

pub mod constants;
pub mod driftwatch;
pub mod model;
pub mod policy_tests;
pub mod redaction;

// Re-export all types
pub use constants::*;
pub use driftwatch::*;
pub use model::*;
pub use redaction::*;
