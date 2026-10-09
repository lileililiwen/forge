//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

/// Contract data version for the API surface. The version
/// is the source of truth for `/healthz` and the response
/// envelope; an older client can refuse the version
/// mismatch instead of silently reinterpreting the
/// response.
pub const API_CONTRACT_VERSION: &str = "0.1.0";

/// Default bind address. Loopback-only so an
/// unauthenticated public listener is impossible by
/// construction.
pub const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Default port the API server binds to when the
/// operator does not pass `--port`. The value is the
/// unprivileged 8765 range to avoid colliding with
/// system services.
pub const DEFAULT_PORT: u16 = 8765;

/// Maximum request body size. Anything larger is refused
/// with `413 Payload Too Large` so a malicious caller
/// cannot pin the server to an unbounded memory
/// allocation. 1 MiB is more than enough for the routes
/// in scope (deploys and feature installs carry typed
/// JSON, not artifacts).
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Maximum time a single connection may be held idle
/// while reading. Bounded so a slow-loris client cannot
/// pin a worker thread.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum time a single request handler may run
/// end-to-end. Bounded so a misbehaving adapter (e.g. an
/// external deploy binary that hangs) cannot keep the
/// listener tied up indefinitely.
pub const HANDLER_TIMEOUT: Duration = Duration::from_secs(60);

/// Synthetic project id recorded in the operations table
/// when the API handles a request that is not bound to
/// one specific registered project (for example
/// `POST /v1/projects` for fleet creation). The id is
/// never visible to operators as a registered project.
pub const API_SYNTHETIC_PROJECT: &str = "__api__";
