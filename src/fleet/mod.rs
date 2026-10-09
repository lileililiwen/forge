//! Read-only fleet observation from the external workspace registry
//! (`fleet-registry-observation`).
//!
//! Workspace Governance owns `projects.json`: its schema, adoption
//! decisions and any writes. This module owns only a read-only parser and
//! a projection of that document into timestamped fleet observations. The
//! local SQLite registry stays the only source of truth for managed
//! projects; fleet data is never copied into it and fleet reads are never
//! journaled operations, so staleness is expressed honestly by
//! `observed_at` instead of a drifting mirror table.
//!
//! ## Input contract (schema_version 1)
//!
//! Accepted document shape, mirroring the sibling registry schema:
//!
//! ```json
//! {"schema_version": 1,
//!  "workspace_root": null,
//!  "discovery": {"mode": "...", "unregistered_policy": "...", "exclude": ["..."]},
//!  "projects": [{"id": "...", "path": "...", "profile": "...",
//!                "lifecycle": "...", "adoption": "..."}]}
//! ```
//!
//! Every field the parser must tolerate (recorded by task 1.1): the
//! top-level keys `schema_version`, `workspace_root` (string or null),
//! `discovery` (`mode`, `unregistered_policy`, `exclude`) and `projects`;
//! per entry `id`, `path`, `profile`, `lifecycle` and the optional
//! nullable `adoption` (the real registry carries `"adopted"` or `null`);
//! and any unknown top-level or per-entry fields (tolerated, e.g. the
//! generated-deployment siblings `generated_by` / `generated_at`).
//!
//! `workspace_root` is deliberately never used to relocate the
//! confinement root: the document is untrusted input, and the sibling
//! convention is that the registry stores no machine-specific root (the
//! root is chosen at invocation). The confinement root is therefore the
//! canonicalized directory containing the registry document, and entry
//! paths resolve inside it.
//!
//! ## Rejection taxonomy (`fleet-registry-invalid`)
//!
//! - Report-level refusals (typed error, exit non-zero, never a partial
//!   silent drop): unreadable file, oversized file, malformed JSON,
//!   missing or unknown `schema_version`, non-object `discovery`,
//!   non-array or oversized `projects`, duplicate ids (naming the id).
//! - Per-entry malformed refusals (the entry is excluded and named with
//!   its reason; the remaining valid entries still report): blank or
//!   non-kebab id, oversized id, blank/oversized/absolute-escape path,
//!   path that resolves outside the root through traversal or a symlink,
//!   non-string or blank `profile`/`lifecycle`.

pub mod limits;
pub mod model;
/// Read-only per-project liveness verdicts for a managed fleet
/// (`fleet-liveness-status`).
pub mod online;
pub mod registry;
pub mod registry_tests;

// Re-export all types
pub use limits::*;
pub use model::*;
pub use registry::*;
