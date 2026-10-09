//! Normalized, provenance-bearing web fleet read model used by
//! `GET /v1/admin/projects`.
//!
//! This module owns only aggregation: it combines the Forge-self record, the
//! local registered projects and any explicitly configured external source
//! into one stable JSON envelope. It never mutates, never scans the
//! filesystem beyond an operator-declared source, and never serializes an
//! external absolute path. The browser transport lives in `frontend/`; the
//! API stays JSON-only.
//!
//! The envelope is versioned separately from `API_CONTRACT_VERSION` so the
//! fleet shape can evolve without touching the auth/session contract. Every
//! registered-project field the standalone frontend already reads (`id`,
//! `profile`, `state`, `lifecycle`, `confidence`, `tags`, `evidence`) stays
//! present so existing callers keep working while the new normalized fields
//! (`identity`, `name`, `source`, `source_ref`, `management`, `is_self`,
//! `freshness`, `capabilities`, `conflict`) are added.

pub mod config;
pub mod contract;
pub mod envelope;
pub mod model;
pub mod sources;

// Re-export all types. The glob facade keeps every historical
// `crate::api::fleet::<Item>` path resolving with zero call-site edits.
// Only `load` is referenced outside this directory today; the remaining
// globs are intentionally retained as the stable facade, hence the allow.
// (`mod fleet` is private to `crate::api`, so rustc reports the
// facade-only globs as unused.)
#[allow(unused_imports)]
pub use config::*;
#[allow(unused_imports)]
pub use contract::*;
#[allow(unused_imports)]
pub use envelope::*;
#[allow(unused_imports)]
pub use model::*;
pub use sources::*;
