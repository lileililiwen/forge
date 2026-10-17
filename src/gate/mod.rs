//! Shared gate runtime evidence (`gate-runtime-evidence`).
//!
//! Forge owns Gate plan resolution (`gate.toml`, `.ai-gate/gate.yaml`,
//! rule-pack identity, blocking policy), execution, evidence, and exit
//! semantics. This module owns the public invocation, normalization of the
//! bundled engine's status document into a versioned [`GateEvidence`]
//! record, and revision-bound freshness. Vendored implementation code is
//! internal and creates no project-local DriftWatch state.
//!
//! Dry-run responses are recognized by their rehearsal shape and routed to
//! [`GateOutcome::Rehearsal`], so a plan can never become evidence. Real runs
//! are normalized into Forge's own evidence contract and only Forge's store
//! and operation journal are updated.
//!
//! Classification discipline: a parseable gate status document is evidence
//! whatever the exit code, and a blocked document is a
//! recorded verdict, not an adapter failure. Unparseable output, a
//! timeout or a spawn failure is `Unavailable` and leaves any previously
//! persisted evidence byte-identical. Every captured string passes
//! [`crate::policy::redact_credentials`] and a character bound before it
//! reaches the evidence file, the journal or a display surface.

pub mod contract;
pub mod defaults;
pub mod engine;
pub mod engine_tests;
pub mod evidence;
pub mod model;
pub mod rehearsal;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
pub use rehearsal::*;
