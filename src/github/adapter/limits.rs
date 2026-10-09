//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Versioned request/response contract for the GitHub metadata adapter.
pub const GITHUB_CONTRACT_VERSION: &str = "forge-github-metadata/0.1.0";

/// Default GitHub host. The adapter is expected to derive the API base
/// from the host (e.g. `https://api.github.com` for `github.com`).
pub const GITHUB_DEFAULT_HOST: &str = "github.com";

/// Default adapter binary name. Matches the `forge-analytics-adapter` /
/// `forge-deploy-adapter` pattern.
pub const DEFAULT_GITHUB_BIN: &str = "forge-github-metadata-adapter";

/// Environment variable selecting the adapter binary.
pub const GITHUB_BIN_ENV: &str = "FORGE_GITHUB_BIN";

/// Environment variable holding the GitHub token. The token is read once
/// and is never echoed.
pub const GITHUB_TOKEN_ENV: &str = "FORGE_GITHUB_TOKEN";

/// Per-run adapter timeout. Matches the analytics / deploy / gate
/// adapters; an unresponsive tool cannot hang the catalog.
pub const GITHUB_ADAPTER_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum number of repositories a single `forge project github
/// observe` call may carry. Larger lists are refused so a misconfigured
/// CLI cannot pin the registry to an unbounded scrape loop.
pub const MAX_OBSERVATION_REPOSITORIES: usize = 32;

/// Maximum number of fields a single `forge project github propose`
/// call may carry. The mutation payload stays small on purpose.
pub const MAX_PROPOSED_CHANGES: usize = 16;

/// Maximum number of topics a single observation may carry. The
/// adapter is expected to cap; this is the defensive ceiling.
pub const MAX_TOPICS: usize = 50;

/// Maximum number of languages a single observation may carry.
pub const MAX_LANGUAGES: usize = 50;

/// Maximum number of release tags a single observation may carry.
pub const MAX_RELEASES: usize = 50;

/// Maximum number of workflows a single observation may carry.
pub const MAX_WORKFLOWS: usize = 50;

/// Maximum number of custom properties a single observation may carry.
pub const MAX_CUSTOM_PROPERTIES: usize = 50;

/// Closed set of fields the adapter is allowed to mutate. The package
/// never opens issues, comments or labels through this surface.
pub const ALLOWED_PROPOSED_FIELDS: [&str; 4] = ["topic", "description", "homepage", "language"];
