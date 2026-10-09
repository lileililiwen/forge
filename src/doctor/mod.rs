//! Read-only health and maturity assessment (`doctor-maturity-assessment`).
//!
//! Core owns the typed finding inventory and the evidence-based maturity
//! policy; transports render Core outcomes without reinterpreting them.
//! Every probe is read-only: filesystem checks use metadata/file reads,
//! Git is inspected via argument arrays (never shell), and neither project
//! files nor remotes are modified. Registry access, when the caller
//! supplies it, is limited to a read-only observation used to mark stale
//! results as stale.
//!
//! Finding model (per design): each finding carries a stable rule ID, a
//! `PASS`/`WARN`/`FAIL`/`UNAVAILABLE` status, evidence, applicability and a
//! remediation class (`automatic`, `AI`, `manual`). Unknown or unexecuted
//! checks are reported explicitly and never converted to `PASS`.
//! Configured maturity is an intent, not proof: only evidence grants a
//! level, and stale observations are shown as stale.
//!
//! The [`gaps`](self::gaps) sub-module owns the catalog-projected
//! evidence-gap findings (`project-evidence-gap-assessment`): it reuses
//! the doctor [`Remediation`] enum so the spec router and any doctor
//! consumer share one classification, and it adds the closed
//! `not_applicable` verdict the package requires for inapplicable
//! controls.

pub mod assessment;
pub mod doctor_tests;
pub mod freshness;
pub mod gaps;
pub mod model;
pub mod probes;
pub mod runner;

// Re-export all types
pub use assessment::*;
pub use model::*;
pub use probes::*;
pub use runner::*;
