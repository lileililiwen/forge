//! Portable project inventory and container fleet selection
//! (`forge-independent-project-inventory-fleet`).
//!
//! Forge consumes a versioned normalized project inventory from an
//! explicitly selected local file or external adapter; the
//! inventory is never silently derived from a sibling checkout. The
//! inventory carries every project, including ones that lack a
//! Compose file — `compose_missing`, `invalid`, and
//! `source_unavailable` are reported per project instead of being
//! filtered out. Only `compose_ready` entries start a Mac Docker
//! workload through the existing provider transport.
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
//! `forge publish fleet` reports every entry; only `compose_ready`
//! entries invoke the provider, never the others. Missing Compose
//! and unresolvable sources are explicit fleet outcomes, never
//! silent omissions.
//!
//! ## Routing
//!
//! Each project carries `runtime` and optional `public_http` plus
//! `public_port`. Web runtimes may declare a public HTTP port and
//! receive `<project>.<domain>` routing through the existing
//! Caddy/cloudflare wildcard boundary on the Mac; non-web runtimes
//! run privately regardless of any other flag; databases,
//! Redis, Jenkins and explicit private workers never reach the
//! router. Forge owns the decision; the jenkins-local sibling owns
//! the Caddy renderer and the tunnel refresh.

pub mod classification_display;
pub mod limits;
pub mod model;
pub mod payload;
pub mod pipeline;
pub mod runtime_display;

// Re-export all types
pub use limits::*;
pub use model::*;
pub use pipeline::*;
