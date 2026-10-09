//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the analytics surface. The
/// adapter speaks the same version on its stdout.
pub const ANALYTICS_CONTRACT_VERSION: &str = "0.1.0";

/// Default adapter binary for `inspect` / `metrics` calls.
/// Real provider integration is out of scope; the binary is
/// invoked with argument arrays and a bounded timeout.
pub const DEFAULT_ANALYTICS_BIN: &str = "forge-analytics-adapter";

/// Environment variable selecting the analytics adapter
/// binary. Matches the `FORGE_DEPLOYER_BIN` /
/// `FORGE_DRIFTWATCH_BIN` pattern.
pub const ANALYTICS_BIN_ENV: &str = "FORGE_ANALYTICS_BIN";

/// Per-run adapter timeout. Spawn + bounded wait so an
/// unresponsive tool cannot hang the registry.
pub const ANALYTICS_ADAPTER_TIMEOUT: Duration = Duration::from_secs(15);

/// Default observation window in days when the manifest
/// does not specify one. Bounded between
/// [`MIN_WINDOW_DAYS`] and [`MAX_WINDOW_DAYS`].
pub const DEFAULT_WINDOW_DAYS: u32 = 7;

/// Minimum observation window in days. A shorter window
/// is refused at validation time.
pub const MIN_WINDOW_DAYS: u32 = 1;

/// Maximum observation window in days. A longer window is
/// refused so a single `forge analytics metrics` call
/// cannot ask for an unbounded history.
pub const MAX_WINDOW_DAYS: u32 = 90;

/// Stable provider kinds. Only `unified-content` and
/// `github-analytics` are in the supported set for v0.1;
/// the others stay discoverable through
/// [`parse_provider`] but [`inspect_external_planes`]
/// reports a typed `unavailable` observation so the
/// registry does not silently grow past what is verified.
pub const PROVIDER_UNIFIED_CONTENT: &str = "unified-content";

pub const PROVIDER_GITHUB_ANALYTICS: &str = "github-analytics";

pub const PROVIDER_NOTION: &str = "notion";

pub const PROVIDER_CONFLUENCE: &str = "confluence";

pub const PROVIDER_GITLAB_ANALYTICS: &str = "gitlab-analytics";

pub const PROVIDER_CODEBERG_ANALYTICS: &str = "codeberg-analytics";

/// Maximum number of provider entries the manifest may
/// declare on the content or repository side. Larger lists
/// are refused with the typed `analytics-invalid` code so
/// the manifest stays bounded.
pub const MAX_PROVIDERS_PER_PLANE: usize = 8;

/// Maximum number of project metrics an aggregate report
/// may carry. Larger reports are refused at validation
/// time so a malicious or misconfigured manifest cannot
/// pin the registry to an unbounded scrape loop.
pub const MAX_METRICS_PER_REPORT: usize = 32;

/// Synthetic project id recorded against the `analytics`
/// journal when the surface runs across the whole
/// registry (e.g. `forge analytics metrics --all`). The id
/// keeps the operations table project-agnostic without
/// inventing a user-visible project.
pub const ANALYTICS_SYNTHETIC_PROJECT: &str = "__analytics__";

/// Stable observation statuses. The transport (CLI/MCP)
/// renders these labels verbatim; the Core contract owns
/// the set.
pub const STATUS_AVAILABLE: &str = "available";

pub const STATUS_UNAVAILABLE: &str = "unavailable";

pub const STATUS_DISABLED: &str = "disabled";

pub const STATUS_UNCONFIGURED: &str = "unconfigured";

pub const STATUS_AMBIGUOUS: &str = "ambiguous-mapping";

pub const METRIC_PROJECTS: &str = "projects";

pub const METRIC_AGENTS_RUNNING: &str = "agents-running";

pub const METRIC_SPECS_QUEUED: &str = "specs-queued";

pub const METRIC_QUALITY_HEALTHY: &str = "quality-healthy";

pub const METRIC_QUALITY_WARNINGS: &str = "quality-warnings";

pub const METRIC_QUALITY_FAILURES: &str = "quality-failures";

pub const METRIC_DEPLOYMENT_RUNNING: &str = "deployment-running";

pub const METRIC_DEPLOYMENT_OFFLINE: &str = "deployment-offline";

pub const METRIC_DEPLOYMENT_PENDING: &str = "deployment-pending";

pub const METRIC_REPOSITORY_STARS: &str = "repository-stars";

pub const METRIC_REPOSITORY_STARS_GROWTH: &str = "repository-stars-growth";

pub const METRIC_STATE_PRESENT: &str = "present";

pub const METRIC_STATE_UNAVAILABLE: &str = "unavailable";

pub const METRIC_STATE_STALE: &str = "stale";

pub const METRIC_STATE_MIXED_WINDOWS: &str = "mixed-windows";

pub const WINDOW_INSTANT: &str = "instant";

pub const WINDOW_DEFAULT_DAYS: &str = "{days}-day";
