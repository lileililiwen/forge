//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Versioned contract for the fleet observation surface.
pub const FLEET_CONTRACT_VERSION: &str = "0.1.0";

/// Environment variable naming the workspace registry document when the
/// `--workspace-registry` flag is absent.
pub const WORKSPACE_REGISTRY_ENV: &str = "FORGE_WORKSPACE_REGISTRY";

/// Schema version this parser understands.
pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

/// Default maximum registry age in seconds (matches the docs freshness
/// one-day vocabulary).
pub const DEFAULT_MAX_AGE_SECONDS: i64 = 86_400;

/// Smallest accepted `--max-age` value.
pub const MIN_MAX_AGE_SECONDS: i64 = 1;

/// Largest accepted `--max-age` value (one year).
pub const MAX_MAX_AGE_SECONDS: i64 = 31_536_000;

/// Registry files larger than this refuse as invalid so a runaway or
/// hostile document cannot pin the reader.
pub const MAX_REGISTRY_BYTES: u64 = 1_048_576;

/// Hard bound on the number of `projects` entries in one document.
pub const MAX_REGISTRY_ENTRIES: usize = 1_024;

/// Per-field display bound after redaction.
pub const MAX_FIELD_CHARS: usize = 200;

/// Maximum accepted id length. Fleet ids also surface in portal entries
/// prefixed with `fleet:`, which caps portal ids at 128 chars.
pub const MAX_FLEET_ID_CHARS: usize = 100;

/// Maximum accepted declared path length.
pub const MAX_FLEET_PATH_CHARS: usize = 1_024;
