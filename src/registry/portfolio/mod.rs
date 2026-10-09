//! Portfolio persistence: additive tables plus the read model.
//!
//! Everything here shares the registry's SQLite file and its
//! migration lifecycle but never a row. The split mirrors the
//! product boundary exactly:
//!
//! - User-owned rows (`portfolio_projects`, `portfolio_tags`,
//!   `portfolio_project_tags`, `portfolio_relations`,
//!   `portfolio_goals`, `portfolio_goal_projects`,
//!   `portfolio_reviews`) are editable through the methods below.
//! - Source-owned rows (`portfolio_evidence_snapshots`) are
//!   append-only. There is no update or delete path for a
//!   snapshot: a later observation is a new row, and the read
//!   model picks the newest one per source.
//!
//! Every mutation validates its arguments through
//! [`crate::portfolio`] before touching the database and checks the
//! target project against the canonical registry identity, so a
//! write for an unknown project changes no portfolio state.

pub mod schema;
pub mod snapshots;
pub mod store;
pub mod validate;
pub mod writes;

// Re-export all types
pub use schema::*;
pub use writes::*;
