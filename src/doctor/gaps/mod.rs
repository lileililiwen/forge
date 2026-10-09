//! Evidence-backed project findings
//! (`project-evidence-gap-assessment`).
//!
//! The gaps package is a **projection** over the catalog contract: it
//! consumes [`CatalogRecord`]s the catalog sources have already produced
//! and emits one typed finding per (category, subject) per record. A
//! finding is an observation, never a stored verdict promoted to
//! health, and the package is strictly read-only: it does not write any
//! file, registry byte, table row, journal row, provider value or CI
//! execution. Repair is the responsibility of a separate package
//! (`project-local-remediation-plans`).
//!
//! ## Vocabularies
//!
//! - [`GapStatus`] is the closed set of verdicts: `pass`, `warn`, `fail`,
//!   `unavailable`, `not_applicable`. An unavailable source is never
//!   reported as healthy by silence; a not-applicable control is
//!   excluded from any healthy-or-failed count.
//! - [`GapCategory`] is the closed set of categories: `description`,
//!   `tags`, `ci`, `compose`, `manifest`, `docs`, `repository`.
//! - [`RemediationClass`] is the spec's `automatic | semantic | manual`
//!   vocabulary; the existing [`crate::doctor::Remediation`] enum is
//!   reused for serialization (`Automatic → automatic`, `Ai → semantic`,
//!   `Manual → manual`).
//!
//! ## Finding id
//!
//! The finding id is stable and derived from `(category, project_id,
//! subject)`, in the form `gaps.<category>.<project_id>.<subject>`. A
//! duplicate `project_id` across two selected sources is rendered twice
//! (the catalog contract never merges records silently), and the source
//! label travels inside the finding's evidence so a downstream consumer
//! can attribute it.

pub mod category_display;
pub mod contract;
pub mod model;
pub mod remediation_display;
pub mod report;
pub mod rule_result;
pub mod rules;
pub mod status_display;

// Re-export all types
pub use contract::*;
pub use model::*;
pub use report::*;
