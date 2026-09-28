//! In-process portal UI for `forge api serve`.
//!
//! Served through `GET /ui`, `GET /ui/projects/{id}`, and
//! `POST /ui/projects/{id}/publish`. Rendered with
//! [`maud`](https://docs.rs/maud) — compile-time, JSX-shaped
//! HTML — so the pages are pure functions of typed data and
//! every project-controlled string flows through maud's
//! default escaping.
//!
//! Modules:
//!
//! - [`auth`]: bearer-token re-check + cross-origin `Origin`
//!   check
//! - [`data`]: registry loaders (fleet list, project detail,
//!   publish plan)
//! - [`render`]: maud templates + typed page data
//! - [`routes`]: the three HTTP handlers

pub mod auth;
pub mod data;
pub mod render;
pub mod routes;
