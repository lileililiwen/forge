//! Semantic component registry and evidence-backed quality levels
//! (`semantic-component-registry`).
//!
//! Core owns the versioned component catalog, the contract validator, the
//! quality-aware resolver, and the evidence-gated promotion path. Transports
//! render Core outcomes without reinterpreting them.
//!
//! The component catalog is built around four rules that distinguish a
//! "semantic component" from a programming primitive
//! (requirement.md §11, §12, §45, §47):
//!
//! 1. **Semantic purpose.** The asset has a meaningful intent
//!    (e.g. `paginated-query`, `idempotency-guard`); language primitives
//!    such as `if`, `loop`, `try-catch`, `string-concat` are refused.
//! 2. **Explicit contract.** The asset names typed inputs and outputs
//!    that callers depend on; the contract is the shared surface every
//!    stack implements independently.
//! 3. **Versioned, testable, deterministic install.** Every descriptor
//!    pins a version, references the test surface, and ships a
//!    deterministic install strategy compatible with the MVP profile
//!    catalog.
//! 4. **Evidence-backed quality.** A quality classification
//!    (`Experimental` / `Verified` / `Certified` / `Deprecated`) is
//!    attached to verifiable evidence: usage count, test coverage,
//!    last verification timestamp, known issues and a security
//!    review flag. Promotion to `Certified` requires the evidence to
//!    be complete; otherwise the prior quality level is preserved and
//!    the request is refused.
//!
//! The resolver prefers the compatible `Certified` candidate whenever
//! multiple components satisfy a request; if no `Certified` candidate
//! exists but `Experimental` and `Verified` candidates do, the
//! strongest available compatible quality wins. A request that only a
//! `Deprecated` or profile-incompatible `Certified` candidate can
//! satisfy surfaces a `component-quality-conflict` rejection so the
//! caller sees the policy conflict instead of a silent selection.
//!
//! Per-stack implementations preserve their own implementation while
//! exposing the shared contract (R1 boundary scenario): the same
//! `paginated-query` id can be installed for `rust-web` and
//! `python-service` with two distinct stack-specific descriptors
//! sharing one contract surface.
//!
//! Storage layout (under the project root, only used by `qualify` and
//! the journal):
//!
//! ```text
//! .forge/components/<component-id>/qualify.json    per-promotion evidence
//! ```
//!
//! Operations journal under the registry's `operations` table with the
//! `component` kind and a `done`/`blocked`/`rejected` verdict so a
//! future portal or API surface can read the history through the same
//! core contract the CLI uses.

pub mod catalog;
pub mod contract;
pub mod engine;
pub mod engine_tests;
pub mod model;

// Re-export all types
pub use catalog::*;
pub use contract::*;
pub use engine::*;
pub use model::*;
