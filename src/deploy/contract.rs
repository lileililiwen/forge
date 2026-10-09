//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

/// Contract data version for the deploy surface. The plan,
/// report and persisted state carry this version.
pub const DEPLOY_CONTRACT_VERSION: &str = "0.1.0";

/// Contract discriminator for the deploy *executor* boundary
/// (`jenkins-deploy-adapter-consumption`). The adapter
/// speaks this id over its stdin payload and stdout
/// envelope; it is frozen in
/// `docs/adapter-contracts/deploy-executor.md`. A third-party
/// adapter emits this exact string; anything else (including
/// the bare `0.1.0` pre-namespacing value) is refused as a
/// contract mismatch so a stale executor can never masquerade
/// as conformant.
pub const DEPLOY_EXECUTOR_CONTRACT: &str = "forge-deploy-executor/0.1.0";

/// Deploy state subdirectory inside the project. Each deploy
/// owns `<project>/.forge/deploy/<project-id>/<deploy-id>/state.json`.
pub const DEPLOY_STATE_DIR: &str = ".forge/deploy";

/// Default adapter binary for `local` and `docker-compose`
/// targets. Real provider integration is out of scope; the
/// binary is invoked with argument arrays and a bounded
/// timeout.
pub const DEFAULT_DEPLOYER_BIN: &str = "forge-deployer";

/// Environment variable selecting the deploy adapter binary.
pub const DEPLOYER_BIN_ENV: &str = "FORGE_DEPLOYER_BIN";

/// Per-run adapter timeout. Spawn plus bounded wait so an
/// unresponsive target cannot hang the registry.
pub const DEPLOY_ADAPTER_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable target kinds. `mac-runtime` is executed by a Linux-side adapter;
/// the target host receives runtime operations only. `ssh` remains planned.
pub const TARGET_LOCAL: &str = "local";

pub const TARGET_DOCKER_COMPOSE: &str = "docker-compose";

pub const TARGET_SSH: &str = "ssh";

pub const TARGET_JENKINS: &str = "jenkins";

/// Linux-controlled runtime target. The adapter runs on the controller and
/// sends only runtime operations to the target host.
pub const TARGET_MAC_RUNTIME: &str = "mac-runtime";

/// Stable health check kinds. `docker` checks a Compose
/// service is running, `http` probes a URL, `process`
/// checks a process exists.
pub const HEALTH_DOCKER: &str = "docker";

pub const HEALTH_HTTP: &str = "http";

pub const HEALTH_PROCESS: &str = "process";

/// Stable per-stage statuses. The transport (CLI/MCP) renders
/// these labels verbatim; the Core contract owns the set.
pub const STATUS_PENDING: &str = "pending";

pub const STATUS_RUNNING: &str = "running";

pub const STATUS_FAILED: &str = "failed";

pub const STATUS_UNKNOWN: &str = "unknown";

pub const STATUS_DISABLED: &str = "disabled";

pub const STATUS_DELIVERED: &str = "delivered";

pub const STATUS_SKIPPED: &str = "skipped";
