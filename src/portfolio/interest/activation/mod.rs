//! Portfolio activation readiness: a read-only, refusal-first verdict
//! over the persisted aggregate interest evidence.
//!
//! This module owns *the gate, not the activation*. Activation is owned
//! by one selected product and consumes this readiness verdict as its
//! input. Forge adds a verdict, a threshold vocabulary and a gate exit
//! code — and no billing, subscription, entitlement, checkout, CRM or
//! revenue-attribution behaviour anywhere.
//!
//! The rules this module enforces:
//!
//! - **Read-only.** No stored state, no table, no migration. The
//!   verdict is a projection over snapshots the interest package
//!   already imported.
//! - **Refusal-first.** Absence of evidence, inexact privacy, partial
//!   coverage, staleness, a missing metric window and a missing or
//!   unmet threshold each withhold readiness with their own named
//!   reason, in a fixed order. Nothing is collapsed into a score, a
//!   percentage or a ranking.
//! - **Latest window only.** Readiness evaluates the latest reported
//!   window for the declared metric and never falls back: falling back
//!   would present older evidence as current readiness.
//! - **`paid_interest_events` is an aggregate signal.** It is never a
//!   payment record, never a grant of access, and grants nothing here.

pub mod contract;
pub mod evaluation;
pub mod readiness;

// Re-export all types
pub use contract::*;
pub use evaluation::*;
pub use readiness::*;
