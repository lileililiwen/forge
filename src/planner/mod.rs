//! Validated intent boundary and reviewable deterministic plan
//! (`validated-intent-planner`).
//!
//! Core owns the versioned Intent contract, the validator that turns a
//! structured model output into a `ValidatedIntent` before any project
//! mutation, the deterministic resolver that compiles the validated
//! intent into a reviewable pinned assembly plan, and the executor
//! that re-binds the plan to the current catalog and re-applies the
//! underlying Core contracts (`feature`, `component`, `ui-pattern`,
//! `doctor`, `gitops`, `policy`, `spec`).
//!
//! AI output is untrusted structured Intent (requirement.md §15, §16,
//! §17, §18, §45, §46). The validator refuses intents that would
//! create an incompatible capability graph — for example
//! `flutter-app + server-postgres` — and explains the compatible
//! client/backend boundary the planner recommends (R1 failure).
//! Ambiguous requests, where the model omitted a required
//! architectural choice, are surfaced as a typed `IntentAmbiguous`
//! rejection so the planner never silently selects a profile or
//! capability (R1 boundary).
//!
//! The deterministic resolver pins every step in the assembly plan to
//! the catalog version captured at validation time so a later
//! `apply_plan` can detect drift: a different profile version, a
//! different feature/component/ui-pattern catalog hash, or a changed
//! working-tree revision is reported as a typed `PlanStale` error
//! and the prior project state is left untouched (R2 failure). When
//! a requirement has no deterministic component, the plan records a
//! bounded `unresolved` entry (a semantic spec id, a glue/business
//! placeholder) instead of inventing an asset; the executor skips
//! those steps so a missing deterministic part is observable rather
//! than masked by an AI substitution (R2 boundary).
//!
//! Operations journal under the registry's `operations` table with
//! the `planner` kind and a `done` / `rejected` / `partial` verdict
//! so a future portal or API surface can read the planner history
//! through the same core contract the CLI uses.

pub mod apply;
pub mod contract;
pub mod model;
pub mod render;
pub mod resolve;
pub mod validate;

// Re-export all types
pub use apply::*;
pub use contract::*;
pub use model::*;
pub use render::*;
pub use resolve::*;
pub use validate::*;
