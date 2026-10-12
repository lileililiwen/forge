//! Shared gate runtime evidence (`gate-runtime-evidence`).
//!
//! Driftwatchdog's Gate owns plan resolution (`gate.toml`,
//! `.ai-gate/gate.yaml`, rule-pack identity, blocking policy), child
//! execution, its own `gate_runs` history and exit semantics. This module
//! owns only the Forge side of that boundary: resolving which runtime a
//! project declares, one bounded argument-array invocation, normalization
//! of the returned status document into a versioned [`GateEvidence`]
//! record, and revision-bound freshness. Forge never re-decides blocking
//! policy: the sibling says passed/blocked, Forge reports that
//! attribution.
//!
//! Live sibling evidence (`tests/fixtures/gate/NOTES.md`) recorded the
//! dry-run surface as prose: at `25811ed` `gate --dry-run` printed a
//! human-readable plan and exited before the JSON writer, so a rehearsal
//! could only ever be a plan preview. The sibling's `gate-dry-run-json-plan`
//! change (`02a13d4`, `447d1b0`) gave that surface a **third document
//! kind** — a machine-readable rehearsal of the resolved plan, composed
//! without executing anything. [`rehearsal`] consumes it: recognised by
//! the document's own shape (never by the runtime's version, which did not
//! change when the shape did), rendered as the resolved plan, and routed
//! to [`GateOutcome::Rehearsal`] so it can never become evidence. A
//! rehearsal answer Forge does not recognise still degrades to the
//! bounded plan-line preview. The evidence surface remains the real
//! `gate --format json` run (whose only side effect is one `gate_runs`
//! row in the project's own `.driftwatch/` store — sibling state, never
//! Forge registry state).
//!
//! Classification discipline (shared with the policy plane): a parseable
//! gate status document is evidence whatever the exit code — the sibling
//! exits non-zero exactly when blocked, and a blocked document is a
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
