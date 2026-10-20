//! Portable project inventory and container fleet selection
//! (`forge-independent-project-inventory-fleet`).
//!
//! Forge consumes a versioned normalized project inventory from an
//! explicitly selected local file or external adapter; the
//! inventory is never silently derived from a sibling checkout. The
//! inventory carries every project, including ones that lack a
//! Compose file — `compose_missing`, `invalid`, and
//! `source_unavailable` are reported per project instead of being
//! filtered out.
//!
//! ## Runtime kinds and the host model
//!
//! Every entry declares a runtime kind. Forge Core owns that
//! vocabulary in [`crate::runtime`], and every kind declares how its
//! artifact is hosted:
//!
//! - `web`, `worker` and `job` are **own-process** kinds: the
//!   artifact runs as a process of its own, with its own container and
//!   its own deploy target.
//! - `library` is the **shared-host** kind: the artifact is a module
//!   loaded into a host another project owns, and several modules can
//!   share one host.
//!
//! ## The publishing rule
//!
//! An entry is published when it is compose-ready **and** its runtime
//! kind is an own-process kind. A shared-host entry is reported and
//! never published, whatever Compose file it declares, because the
//! host that adopts the module owns its lifecycle. Every reported row
//! whose runtime kind was read names its host model, and a shared-host
//! row carries the reason that it is a module hosted by the project
//! that adopts it. A row whose runtime kind could not be read names
//! neither: it carries the refusal and nothing else.
//!
//! The rule lives in one function — `is_publishable` — and the
//! observe lane (`forge fleet online`) reads it too, so a module the
//! fleet never started is never probed.
//!
//! ## Contract (`forge-project-inventory/0.1.0`)
//!
//! ```json
//! {
//!   "contract": "forge-project-inventory/0.1.0",
//!   "provider": "local",
//!   "generated_at": "RFC3339",
//!   "projects": [{
//!     "id": "example",
//!     "repository": "https://github.com/org/example.git",
//!     "revision": "40-hex-sha",
//!     "profile": "rust-product",
//!     "runtime": "web|worker|job|library",
//!     "compose_file": "docker-compose.yml",
//!     "source_path": "/invocation/only/example",
//!     "public_http": true,
//!     "public_port": 8080
//!   }]
//! }
//! ```
//!
//! `source_path` is never persisted as the project identity; it
//! resolves the staged source tree when Forge needs to compose it
//! through a local provider.
//!
//! ## Fleet classification
//!
//! Every declared project receives one explicit classification:
//!
//! - `compose_ready`     — `compose_file` exists at `source_path`.
//! - `compose_missing`   — `compose_file` is null or absent.
//! - `invalid`           — contract validation refused the entry.
//! - `source_unavailable`— `source_path` does not resolve on this host.
//!
//! The classification is a fact about the declared source path, not a
//! verdict on the entry. `forge publish fleet` reports every entry;
//! only the compose-ready entries whose kind is an own-process kind
//! invoke the provider, never the others. Missing Compose and
//! unresolvable sources are explicit fleet outcomes, never silent
//! omissions.
//!
//! ## Routing
//!
//! Each project carries `runtime` and optional `public_http` plus
//! `public_port`. Web runtimes may declare a public HTTP port and
//! receive `<project>.<domain>` routing through the existing
//! Caddy/cloudflare wildcard boundary on the Mac; non-web runtimes
//! run privately regardless of any other flag; databases,
//! Redis, Jenkins and explicit private workers never reach the
//! router. A shared-host module may not expose HTTP at all, and the
//! refusal names the host model. Forge owns the decision; the
//! jenkins-local sibling owns the Caddy renderer and the tunnel
//! refresh.

pub mod classification_display;
pub mod limits;
pub mod model;
pub mod payload;
pub mod pipeline;

// Re-export all types
pub use limits::*;
pub use model::*;
pub use pipeline::*;
