//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::sync::LazyLock;
use std::time::Duration;

use crate::extension::version::ContractVersion;

/// The deploy target kinds live in the shared extension module, so
/// the manifest validator, the configuration validator and the
/// deployment plan read one list. They are re-exported here so
/// `crate::deploy::TARGET_LOCAL` keeps resolving.
pub use crate::extension::kinds::{
    TARGET_DOCKER_COMPOSE, TARGET_JENKINS, TARGET_LOCAL, TARGET_MAC_RUNTIME, TARGET_SSH,
};

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

/// The executor contract, read into the shared version type once.
static DEPLOY_EXECUTOR_CONTRACT_VERSION: LazyLock<ContractVersion> = LazyLock::new(|| {
    ContractVersion::parse(DEPLOY_EXECUTOR_CONTRACT)
        .expect("the deploy executor contract constant is a well-formed contract version")
});

/// The deploy executor contract the adapter must satisfy: the
/// `forge-deploy-executor` family at the required major. A newer
/// minor of the same family satisfies it too; the shared rule
/// decides.
pub fn deploy_executor_contract() -> ContractVersion {
    DEPLOY_EXECUTOR_CONTRACT_VERSION.clone()
}

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
