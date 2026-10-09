//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Versioned contract for the project-inventory surface.
pub const INVENTORY_CONTRACT_VERSION: &str = "forge-project-inventory/0.1.0";

/// Environment variable naming the inventory source when the CLI
/// flag is absent. Defaults to local-only — no implicit sibling
/// search.
pub const INVENTORY_SOURCE_ENV: &str = "FORGE_INVENTORY_SOURCE";

/// Bounded maximum project entries accepted by one inventory
/// document. Larger documents refuse as invalid so a runaway
/// adapter cannot pin the reader.
pub const MAX_INVENTORY_ENTRIES: usize = 1_024;

/// Bounded maximum document size in bytes.
pub const MAX_INVENTORY_BYTES: u64 = 4_096_000;

/// Bounded inventory adapter subprocess timeout (seconds). Generous
/// enough for a remote clone, bounded so a hostile adapter cannot
/// pin Forge indefinitely.
pub const INVENTORY_ADAPTER_TIMEOUT_SECS: u64 = 300;

/// Maximum length of an id or runtime class string.
pub const MAX_ID_CHARS: usize = 128;

pub const MAX_RUNTIME_CHARS: usize = 32;

/// Per-field display bound after redaction.
pub const MAX_FIELD_CHARS: usize = 200;

/// Public domain used for `<project>.<domain>` routing when the
/// inventory declares `public_http = true`.
pub const DEFAULT_DOMAIN: &str = "tooosall.uk";
