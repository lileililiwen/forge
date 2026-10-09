//! Optional control-plane portal data surface
//! (`control-plane-portal`).
//!
//! Forge Core owns the typed portal contract. The portal
//! is a *read-only* view layer that aggregates the data
//! every other Core module already publishes (registry,
//! doctor, features, components, specs, agents, deploys,
//! distribution, docs, analytics, identity) into a single
//! stable, versioned JSON envelope that a future portal
//! framework (ASP.NET Core or Next.js, per the design ADR)
//! can render. Core returns typed outcomes; the portal
//! never reinterprets Core state.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §36 calls for an
//! optional web portal that surfaces the shared control
//! plane. The brief explicitly states the portal should
//! "consume the same Forge Core APIs as CLI and MCP", so
//! the portal is a thin view of the Core registry plus
//! the timestamped observations every other module
//! already journals. No business rule is duplicated: the
//! portal is a renderer, not a reimplementation.
//!
//! ## Boundary
//!
//! - **Read-only projection.** The portal surface never
//!   mutates the manifest, the registry, the feature
//!   receipts, the spec proposals, the deploy state, the
//!   distribution state, the docs derivatives or the
//!   identity sessions. The CLI and the mature MCP tools
//!   remain the only mutating transports; the portal
//!   surfaces a `controls_available` line so the operator
//!   sees where the same operation can be invoked.
//! - **Evidence is timestamped and versioned.** Every
//!   portal view carries the `contract` version, a
//!   `generated_at` timestamp and a `source` field
//!   (registry / doctor / feature / …) so a stale or
//!   unknown observation is never silently re-rendered as
//!   success. A dashboard can imply stronger health than
//!   the evidence supports; the contract surfaces the
//!   `unknown` / `stale` / `partial` states explicitly
//!   (R3 boundary).
//! - **No implicit remote write.** The portal is
//!   loopback-friendly: it never reaches a remote API,
//!   never invokes an external adapter, never authenticates
//!   against an OIDC provider. Authorization is performed
//!   by the underlying Core contract; the portal is just a
//!   viewer.
//! - **Bounded sections.** The portal ships the twelve
//!   named sections listed in §36. Unknown sections are
//!   refused at validation time so the CLI cannot silently
//!   ask for a view the registry cannot answer.
//!
//! ## Persistence
//!
//! No new persistent state is added: the portal reads the
//! manifest, the registry's `projects` and `operations`
//! tables, the doctor report, the per-module state files
//! and the per-module journals. The Core registry's
//! `operations` table receives one `portal` row per
//! `dashboard` / `view` call with a `done` / `partial`
//! verdict and the project id (or the synthetic
//! `__portal__` project id when the view spans the entire
//! registry).
//!
//! ## Risk model
//!
//! A real portal framework is out of scope for the local
//! sandbox: the contract is validated through the
//! `forge portal dashboard` / `forge portal view <section>`
//! CLI surface and the typed `portal-invalid` error. The
//! JSON envelope is the contract a future ASP.NET Core or
//! Next.js renderer would consume. A real portal
//! integration is a downstream step.

pub mod activity;
pub mod constants;
pub mod model;
pub mod portal_tests;
pub mod render;
pub mod sections;

// Re-export all types
pub use activity::*;
pub use constants::*;
pub use model::*;
pub use render::*;
pub use sections::*;
