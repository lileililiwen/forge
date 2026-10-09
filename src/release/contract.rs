//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the release surface. The package
/// and container adapters speak the same version on
/// stdin/stdout.
pub const RELEASE_CONTRACT_VERSION: &str = "0.1.0";

/// Release state subdirectory inside the project. Each release
/// owns `<project>/.forge/release/<project-id>/<release-id>/state.json`.
pub const RELEASE_STATE_DIR: &str = ".forge/release";

/// Default changelog filename when the manifest omits one.
pub const DEFAULT_CHANGELOG: &str = "CHANGELOG.md";

/// Default release notes template when the manifest omits one.
pub const DEFAULT_NOTES_TEMPLATE: &str = "RELEASE_NOTES.md";

/// Default release notes output when the manifest omits one.
pub const DEFAULT_NOTES_OUTPUT: &str = "RELEASE_NOTES.md";

/// Default binary for the package adapter. Real provider
/// integration is out of scope; the binary is invoked with
/// argument arrays and a bounded timeout.
pub const DEFAULT_PACKAGE_BIN: &str = "forge-package-publisher";

/// Environment variable selecting the package adapter binary.
pub const PACKAGE_BIN_ENV: &str = "FORGE_PACKAGE_BIN";

/// Default binary for the container adapter.
pub const DEFAULT_CONTAINER_BIN: &str = "forge-container-publisher";

/// Environment variable selecting the container adapter binary.
pub const CONTAINER_BIN_ENV: &str = "FORGE_CONTAINER_BIN";

/// Default binary for the release notes adapter.
pub const DEFAULT_NOTES_BIN: &str = "forge-notes-renderer";

/// Environment variable selecting the release notes adapter.
pub const NOTES_BIN_ENV: &str = "FORGE_NOTES_BIN";

/// Per-run adapter timeout. Spawn plus bounded wait so an
/// unresponsive provider cannot hang the registry.
pub const ADAPTER_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable stage identifiers. Stages are applied in this
/// declaration order; the contract never reorders a stage
/// after a release identity is captured.
pub const STAGE_COMMIT: &str = "commit";

pub const STAGE_TAG: &str = "tag";

pub const STAGE_PUSH: &str = "push";

pub const STAGE_MIRROR: &str = "mirror";

pub const STAGE_PACKAGE: &str = "package";

pub const STAGE_CONTAINER: &str = "container";

pub const STAGE_DOCS: &str = "docs";

pub const STAGE_NOTES: &str = "notes";

/// Default stage list when the manifest does not override it.
pub const DEFAULT_STAGES: &[&str] = &[
    STAGE_COMMIT,
    STAGE_TAG,
    STAGE_PUSH,
    STAGE_MIRROR,
    STAGE_PACKAGE,
    STAGE_CONTAINER,
    STAGE_DOCS,
    STAGE_NOTES,
];

/// Stable per-stage statuses. The transport (CLI/MCP) renders
/// these labels verbatim; the Core contract owns the set.
pub const STATUS_SKIPPED: &str = "skipped";

pub const STATUS_DELIVERED: &str = "delivered";

pub const STATUS_FAILED: &str = "failed";

pub const STATUS_DISABLED: &str = "disabled";

pub const STATUS_DIVERGED: &str = "diverged";

pub const STATUS_UNAVAILABLE: &str = "unavailable";

pub const STATUS_CONFLICT: &str = "conflict";

/// Stable per-check statuses.
pub const CHECK_PASS: &str = "pass";

pub const CHECK_FAIL: &str = "fail";

pub const CHECK_UNAVAILABLE: &str = "unavailable";

pub const CHECK_STALE: &str = "stale";

pub const CHECK_DISABLED: &str = "disabled";
