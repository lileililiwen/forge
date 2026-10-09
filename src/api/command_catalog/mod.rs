//! forge — typed CLI command-catalog metadata (`forge-command-catalog/0.1.0`).
//!
//! The catalog is the browser-facing inventory of every Rust CLI command in
//! [`crate::Cli`]: one row per Clap path (top-level and nested), each mapped
//! to a truthful availability state. It is metadata only — the API serializes
//! descriptions, never executes commands, and exposes no shell/eval route.
//! The parity test inside `src/main.rs` walks the real Clap tree and fails
//! when this catalog and the CLI drift apart.
//!
//! Availability vocabulary (design.md): `web`, `cli_only`,
//! `provider_required`, `project_capability_required`, `disabled`,
//! `not_yet_web`. Every non-web row carries a plain-language reason and next
//! step; `web` rows must resolve to an implemented typed JSON route. Risk
//! labels are guidance for reading, not a permission grant: authorization is
//! enforced by each invoked operation.

pub mod builder;
pub mod catalog;
pub mod model;
pub mod routes;
pub mod rows_assurance;
pub mod rows_fleet;
pub mod rows_platform;
pub mod rows_project;
pub mod rows_shipping;

// Re-export all types
pub use catalog::*;
pub use model::*;
pub use routes::*;
