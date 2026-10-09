//! Optional HTTP transport over Core (`core-http-api`).
//!
//! Forge Core owns every domain rule; this module is a thin
//! HTTP/1.1 transport that exposes the stable project and
//! lifecycle operations listed in [requirement.md §35]. The
//! API server defaults to a loopback-only bind so an
//! unauthenticated public listener is impossible by
//! construction; production exposure is the responsibility
//! of the operator and is out of scope for v0.1.
//!
//! ## Why
//!
//! The brief calls for an optional HTTP API that lets
//! callers reach the same Core contracts the CLI and MCP
//! surfaces consume. The same typed outcomes must flow
//! through every transport: the API validates input, then
//! dispatches through the existing `forge::core` modules
//! (registry, doctor, feature, upgrade, spec, agent,
//! deploy) and renders the response. No business rule is
//! duplicated in the transport.
//!
//! ## Authorization and idempotency
//!
//! Every request (other than `GET /healthz`) requires an
//! `Authorization: Bearer <session-id>` header. The token
//! is an OIDC admin session minted by the
//! `central-admin-identity` surface; it lives at
//! `.forge/identity/<project>/sessions/<id>.json` and is
//! strictly project-scoped. A token minted for project A
//! cannot authorize a request against project B; the
//! request is refused with
//! [`ForgeError::ApiProjectMismatch`] (R2 failure
//! scenario). Mutating routes additionally require the
//! session to carry the `admin:access` permission; a
//! session without that permission is refused with
//! [`ForgeError::ApiUnauthorized`].
//!
//! Mutating routes accept an `Idempotency-Key` header so
//! retries are safe. The `(kind, key)` pair is unique in
//! the registry's operations table; an identical retry
//! reuses the original `op_id` (R2 boundary scenario:
//! external side effects are not repeated). A retry that
//! reuses the key with a different request body is
//! refused with [`ForgeError::IdempotencyKeyConflict`] so
//! the operator never silently reinterprets a prior
//! operation.
//!
//! ## Async operation model
//!
//! Every mutating route returns `202 Accepted` with a
//! `Location: /v1/operations/<id>` header and a JSON
//! envelope containing the operation's recorded state.
//! The operation is journaled in the same `operations`
//! table every other transport writes to; the API layer
//! is therefore a peer of the CLI and MCP journals. A
//! `GET /v1/operations/<id>` request returns the latest
//! state so a caller can poll for progress or terminal
//! outcomes.
//!
//! ## Risks
//!
//! Binding HTTP broadens access beyond a local process.
//! This module therefore defaults to `127.0.0.1` and
//! refuses non-loopback binds unless the operator passes
//! `--bind 0.0.0.0` explicitly. Authentication is required
//! on every route other than the loopback health check;
//! the loopback health check is the only anonymous
//! surface and only returns `200 ok` plus the contract
//! version.

mod admin;
#[cfg(test)]
pub mod api_tests;
/// Typed CLI command-catalog metadata (`forge-command-catalog/0.1.0`)
/// backing `GET /v1/admin/commands`. Metadata only: the catalog never
/// executes anything and the API exposes no shell/eval route. Named
/// `command_catalog` because `catalog` already refers to the project
/// catalog (`forge-project-catalog/0.1.0`) in this module.
pub mod command_catalog;
pub mod contract;
/// Session-gated, confirm- and digest-bound delivery controls
/// (`forge-web-delivery-controls/0.1.0`) backing `/v1/admin/delivery*`:
/// the share allowlist → preview → approve → publish pipeline through the
/// crate's typed in-process Core functions. Every mutation requires an
/// explicit confirmation bound to the reviewed manifest digest; the
/// browser never supplies a path, and the subprocess publication adapter
/// stays CLI-only.
mod delivery;
mod fleet;
/// Read-only fleet list loaders used by `src/api/fleet.rs`. The
/// loader was previously part of the in-process portal UI data
/// path (`src/api/ui/data.rs`); it survives the removal because
/// the JSON API fleet projection depends on the typed view
/// structs it returns.
mod fleet_data;
pub mod handlers_changes;
pub mod handlers_core;
pub mod handlers_portfolio;
pub mod handlers_studio;
/// Web lifecycle execution (`web-lifecycle-execution/0.1.0`): graduation,
/// intent, remediate and next-idea preview + confirm/digest-bound routes
/// through the same in-process Core functions the CLI runs.
mod lifecycle_exec;
/// Per-project maintainer surface (`forge-project-maintain/0.1.0`) backing
/// `GET /v1/admin/projects/{id}/maintain` and the `classify/approve`,
/// `classify/reject` and `classify/apply` preview → confirm → apply routes.
/// Read-only observation plus typed, digest-bound decisions only — the
/// browser addresses a project by validated id and never sends a path.
mod maintain;
pub mod model;
/// Session-gated portfolio controls and cross-project evidence views
/// (`forge-web-portfolio-controls/0.1.0`) backing `/v1/admin/portfolio*`.
/// Forge-owned metadata writes reuse registry Core only; imported,
/// source-owned evidence is read-only and never executed live on page load.
mod portfolio;
/// Session-gated project management (`forge-web-project-management`):
/// browser `new`/`import`/`register` plus the shared
/// `FORGE_ADMIN_PROJECTS_ROOT` confinement both this module and
/// [`workspace`](self::workspace) build on.
mod project_management;
pub mod router;
pub mod server;
/// Typed, session-gated read-only status projection (`forge-project-status/
/// 0.1.0`) backing `GET /v1/admin/projects/{id}/status` and
/// `GET /v1/admin/status`. Reuses the in-process doctor, checker and profile
/// readiness projections only — never a shell, a write or an external
/// adapter — and never serializes an absolute filesystem path.
mod status;
/// Typed, session-gated single-project workbench (`forge-project-workbench/
/// 0.1.0`) backing `GET /v1/admin/projects/{id}`, its `/plan` and `/apply`
/// subroutes. Reuses typed in-process Core functions only — never a shell —
/// and never serializes an absolute filesystem path.
mod workbench;
/// Live workspace onboarding (`forge-web-workspace-onboarding`): read-only
/// candidate discovery over the configured project root plus bulk
/// preview/confirm/digest-bound onboarding.
mod workspace;

// Re-export all types
pub use contract::*;
pub use model::*;
pub use router::*;
pub use server::*;
// Scope-preserving re-exports for names the `admin` child (and other
// `api` descendants) address through the historical `crate::api::<Item>`
// path. `pub(in crate::api)` keeps the exact prior visibility: reachable
// anywhere inside `api`, invisible outside it.
pub(in crate::api) use handlers_changes::{
    handle_add_feature, handle_apply_spec, handle_generate_spec, handle_remove_feature,
    handle_upgrade_feature,
};
pub(in crate::api) use router::err_status;
pub(in crate::api) use server::{percent_decode, run_with_operation};
