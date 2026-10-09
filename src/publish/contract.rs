//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract version for the publish surface. The report carries this
/// version so the transport and any later reader share one definition.
pub const PUBLISH_CONTRACT_VERSION: &str = "forge-publish/0.1.0";

/// Subprocess timeout. The Mac may be slow over SSH; the timeout is
/// bounded so an unresponsive host never hangs Forge forever.
pub const PUBLISH_SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(60);

/// Subprocess timeout for stages that build or recreate containers
/// (`docker compose up --build`, router reload). Image builds
/// routinely exceed the interactive 60s bound; the 1800s ceiling
/// matches the external provider invocation bound while still
/// refusing to hang forever.
pub const PUBLISH_DEPLOY_TIMEOUT: Duration = Duration::from_secs(1800);

/// Subprocess timeout for the sync stage (`rsync` of the source
/// tree). Multi-GB trees under parallel-fleet contention routinely
/// exceed the interactive 60s bound; the 600s ceiling keeps genuine
/// stalls failing closed at a documented bound while fast probes
/// keep the 60s default and deploy builds keep the 1800s ceiling
/// (`fleet-live-rollout`).
pub const PUBLISH_SYNC_TIMEOUT: Duration = Duration::from_secs(600);

/// Stable stage names. The transport renders these labels verbatim.
pub const STAGE_SYNC: &str = "sync";

pub const STAGE_DB: &str = "db";

pub const STAGE_PREPARE: &str = "prepare";

pub const STAGE_DEPLOY: &str = "deploy";

/// Stable per-stage statuses.
pub const STATUS_DRY_RUN: &str = "dry-run";

pub const STATUS_DONE: &str = "done";

pub const STATUS_FAILED: &str = "failed";

pub const STATUS_SKIPPED: &str = "skipped";
