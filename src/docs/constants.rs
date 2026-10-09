//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the docs surface. The provider
/// adapter speaks the same version on stdin/stdout.
pub const DOCS_CONTRACT_VERSION: &str = "0.1.0";

/// Docs state subdirectory inside the project. Each locale owns
/// `<project>/.forge/docs/<locale>/state.json`.
pub const DOCS_STATE_DIR: &str = ".forge/docs";

/// Canonical source document when the manifest's `docs.source`
/// is absent.
pub const DEFAULT_SOURCE: &str = "README.md";

/// Default translator binary. Overridable per run through
/// `FORGE_DOCS_TRANSLATOR_BIN` so contract fixtures can stand
/// in for a real provider.
pub const DEFAULT_TRANSLATOR_BIN: &str = "forge-docs-translator";

/// Environment variable selecting the translator binary.
pub const TRANSLATOR_BIN_ENV: &str = "FORGE_DOCS_TRANSLATOR_BIN";

/// Per-run translator timeout. The adapter uses spawn plus a
/// bounded wait so an unresponsive provider cannot hang the
/// registry.
pub const TRANSLATOR_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable per-locale outcome statuses.
pub const STATUS_TRANSLATED: &str = "translated";

pub const STATUS_CURRENT: &str = "current";

pub const STATUS_FAILED: &str = "failed";
