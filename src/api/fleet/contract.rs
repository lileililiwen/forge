//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Versioned contract for the normalized web fleet envelope. Bumped to
/// `0.2.0` when the `published` source and the per-row `publish` object were
/// added; existing fields and sources are unchanged and remain backward
/// compatible.
pub const WEB_FLEET_CONTRACT_VERSION: &str = "forge-web-fleet/0.2.0";

/// Default stable identity for the Forge-self row; overridable through the
/// established `FORGE_SELF_ID` configuration path so an operator can bind
/// the self record to a known registry id without editing code.
pub const SELF_DEFAULT_ID: &str = "forge";

/// Human display name for the Forge-self row.
pub const SELF_NAME: &str = "Forge";

/// Environment variable naming the freshness window (seconds) applied to
/// external sources; validated against the existing fleet bounds.
pub const FLEET_MAX_AGE_ENV: &str = "FORGE_FLEET_MAX_AGE_SECONDS";

/// Environment variable overriding the Forge-self identity.
pub const SELF_ID_ENV: &str = "FORGE_SELF_ID";

/// Environment variable controlling the local publish-history projection.
/// The projection is enabled by default (it reads the Forge-owned local
/// registry, never an external path); `0`, `false` or `off` disables it and
/// reports the `published` source as unconfigured.
pub const PUBLISH_HISTORY_ENV: &str = "FORGE_PUBLISH_HISTORY";

/// Environment variable bounding how many distinct published projects are
/// projected. Parsed as a positive integer and clamped to
/// `1..=PUBLISH_HISTORY_MAX_LIMIT`.
pub const PUBLISH_HISTORY_LIMIT_ENV: &str = "FORGE_PUBLISH_HISTORY_LIMIT";

/// Default bound for the publish-history source.
pub const PUBLISH_HISTORY_DEFAULT_LIMIT: usize = 200;

/// Hard ceiling so a machine-supplied limit can never pin the renderer.
pub const PUBLISH_HISTORY_MAX_LIMIT: usize = 1000;
