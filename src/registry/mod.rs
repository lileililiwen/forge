//! Persistent project registry backed by SQLite.
//!
//! The registry stores one row per project plus an append-only operation
//! journal. Manifest files and the database can never share one
//! transaction, so `register` writes the database only: a `pending`
//! journal entry is committed first, then the project row plus the
//! terminal journal state in a second transaction. A `pending` entry
//! found at open time belongs to an interrupted run and is reconciled to
//! `failed` — it is never reported as success.
//!
//! The portfolio domain shares this file but never a row: its tables
//! live in [`portfolio`] and are created additively by
//! [`PORTFOLIO_SCHEMA_SQL`], and the portfolio share allowlist lives
//! in [`share`] behind [`PORTFOLIO_SHARE_SCHEMA_SQL`], so a registry
//! written before either package keeps every one of its own rows
//! untouched.

mod interest;
pub mod model;
mod portfolio;
pub mod schema;
mod share;
pub mod store;

// Re-export all types
pub use interest::SnapshotOutcome;
pub use model::*;
pub use portfolio::{PortfolioWrite, SnapshotWrite};
pub use schema::*;
pub use share::audit::PublicationReservation;
pub use store::*;
