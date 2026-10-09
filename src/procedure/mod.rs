//! Portable AI procedures over stable Core operations
//! (`ai-procedure-skills`).
//!
//! Core owns the versioned procedure catalog, the
//! [`CoreOperation`] enumeration over the stable Core contracts
//! the procedure steps may reference, and the
//! [`validate_procedure`] function that refuses any step that
//! would either reference an unavailable or unstable operation
//! or smuggle a bypass marker (`--force`, `--skip-checks`,
//! `--no-validate`, `--bypass`, `--override`, `--no-doctor`,
//! `--ignore-failures`) into a Core call. The CLI (`forge
//! procedure list|inspect|validate`) is the first transport;
//! future MCP, API and portal surfaces consume the same Core
//! contract.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §19, §32 require
//! portable AI procedures over stable operations. The
//! catalogued procedures (create-project, upgrade-project,
//! prepare-release, fix-quality-findings,
//! onboard-existing-project, deploy-project,
//! mirror-repository, translate-docs) describe a dependency-
//! ordered sequence of Core operations plus a final
//! `report_findings` step. They are AI SOPs: the steps name
//! the Core operation, the expected inputs and the rationale
//! so a model or a human operator can audit the workflow
//! before it runs.
//!
//! ## Boundary
//!
//! The procedure layer never executes Core operations on
//! its own. Every step points at a stable Core contract
//! (`profile.inspect`, `feature.add`, `doctor.run`, …) so
//! mutation validation, project isolation, path confinement,
//! evidence redaction and redaction rules stay in Core. A
//! procedure whose step carries a bypass marker is refused by
//! [`validate_procedure`] with the typed
//! `procedure-bypass-refused` code (R2 failure scenario): the
//! planner, the model and the operator can ask, but Core does
//! not act on a request to override its own validation.
//!
//! ## Discovery
//!
//! [`procedure_catalog`] returns the eight named procedures in
//! stable id order. [`inspect_procedure`] returns the immutable
//! [`ProcedureSpec`] for the named id. The catalog and the
//! validator carry no agent-provider, IDE or model identifier,
//! so a provider change is a no-op for the procedure layer
//! (R1 boundary scenario): the platform-neutral workflow and
//! the Core contracts do not change.
//!
//! ## Persistence
//!
//! Procedures are versioned static resources in this cycle;
//! no per-project state is written. The Core registry's
//! `operations` table receives one `procedure` row per
//! list/inspect/validate call with a `done` / `rejected`
//! verdict and a synthetic `__procedure__` project id so a
//! future portal surface can read the procedure history
//! through the same Core contract the CLI uses.

pub mod catalog;
pub mod catalog_tests;
pub mod constants;
pub mod model;

// Re-export all types
pub use catalog::*;
pub use constants::*;
pub use model::*;
