//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Versioned request/response contract for the `gh` CLI workflow.
pub const GITHUB_CLI_CONTRACT_VERSION: &str = "forge-github-cli-workflows/0.1.0";

/// Default GitHub host the package uses. The user can override it on
/// the CLI; the adapter passes it to `gh auth status --hostname`.
pub const GITHUB_CLI_DEFAULT_HOST: &str = "github.com";

/// Default `gh` binary name. Matches the `gh` CLI as installed by
/// the user; the binary may be overridden through [`GITHUB_CLI_BIN_ENV`].
pub const DEFAULT_GITHUB_CLI_BIN: &str = "gh";

/// Environment variable selecting the `gh` binary.
pub const GITHUB_CLI_BIN_ENV: &str = "FORGE_GH_BIN";

/// Per-call wall-clock timeout. The `gh` CLI can hang on a stalled
/// network or on a slow `repo view`; an unresponsive tool cannot
/// hang Forge.
pub const GITHUB_CLI_TIMEOUT: Duration = Duration::from_secs(30);

/// Minimum timeout the caller may request. Anything below this is
/// clamped so a misconfigured caller cannot make `gh` unkillable.
pub const GITHUB_CLI_MIN_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum timeout the caller may request.
pub const GITHUB_CLI_MAX_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum number of bytes a `--title` argument may carry.
pub const MAX_TITLE_BYTES: usize = 256;

/// Maximum number of bytes a `--body` argument may carry.
pub const MAX_BODY_BYTES: usize = 8192;

/// Maximum number of bytes a `--destination` path may carry.
pub const MAX_DESTINATION_BYTES: usize = 4096;
