//! Traceable spec generation and remediation routing
//! (`specification-remediation`).
//!
//! Core owns the versioned spec-storage layout, the
//! finding→remediation routing decision, and the bounded proposal
//! emitted by `forge spec generate`. Transports render Core outcomes
//! without reinterpreting them.
//!
//! A spec is always traceable: its [`SpecProvenance`] records the
//! project identity, the source revision (manifest mtime or other
//! deterministic identifier) and the exact finding identifiers that
//! triggered the proposal. Generation is idempotent on the same
//! finding set (same project + same sorted finding identifiers hash
//! to the same spec id), so a repeat run for an unchanged finding
//! reports the existing spec rather than producing a duplicate
//! (boundary scenario).
//!
//! Remediation routing classifies a finding into one of three
//! queues:
//!
//! - `Deterministic` — a supported lifecycle or upgrade action
//!   resolves the finding; the operation runs only after its declared
//!   validators and a follow-up `forge doctor` pass, so the success
//!   scenario records the fix and the evidence.
//! - `Semantic` — a bounded spec must be generated for an agent or a
//!   human to implement; the spec is the handoff.
//! - `Manual` — the finding is a judgment call; the router records
//!   the manual status without claiming an AI fix and without
//!   changing the project (boundary scenario). A failure leaves the
//!   finding unresolved and surfaces the recovery information.
//!
//! Storage layout (under the project root):
//!
//! ```text
//! .forge/specs/
//!   <spec-id>/
//!     manifest.json   machine-readable SpecDraft + provenance
//!     proposal.md     bounded proposal (R1 traceable payload)
//!     design.md       minimal design skeleton
//!     tasks.md        minimal task checklist
//!     routing.json    recorded routing decision when routed
//! ```
//!
//! Spec ids are stable across invocations: `<project-id>-<short-hash>`,
//! where the hash is a content-derived identifier over the sorted
//! finding set. Two requests targeting the same project and the same
//! findings collapse to one spec; a request targeting different
//! projects or incompatible finding sets is refused with
//! `error[spec-ambiguous]` so no misleading combined spec is written.

pub mod contract;
pub mod finding_convert;
pub mod lifecycle;
pub mod lifecycle_tests;
pub mod model;
pub mod routing_serialize;
pub mod spec_id_serialize;

// Re-export all types
pub use contract::*;
pub use lifecycle::*;
pub use model::*;
