//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Adapter id an operator selects or stores in configuration.
pub const ADAPTER_ID: &str = "remote-compose";

/// Adapter id of the retained legacy Mac-script lane.
pub const LEGACY_ADAPTER_ID: &str = "jenkins";

/// Environment variable selecting the publish adapter.
pub const ADAPTER_ENV: &str = "FORGE_PUBLISH_ADAPTER";

/// SSH host alias for the target.
pub const SSH_TARGET_ENV: &str = "FORGE_PUBLISH_SSH_TARGET";

/// Root of the synced project tree on the target.
pub const REMOTE_ROOT_ENV: &str = "FORGE_PUBLISH_REMOTE_ROOT";

/// Root of the per-project runtime tree on the target.
pub const RUNTIME_ROOT_ENV: &str = "FORGE_PUBLISH_RUNTIME_ROOT";

/// Root of the router (Caddy) tree on the target.
pub const PLATFORM_ROOT_ENV: &str = "FORGE_PUBLISH_PLATFORM_ROOT";

/// Root of the target-local secret tree on the target.
pub const SECRETS_ROOT_ENV: &str = "FORGE_PUBLISH_SECRETS_ROOT";

/// Root of the shared-infrastructure Compose tree on the target.
pub const SHARED_INFRA_ROOT_ENV: &str = "FORGE_PUBLISH_SHARED_INFRA_ROOT";

/// Public domain used for `<project>.<domain>` routing.
pub const DOMAIN_ENV: &str = "FORGE_PUBLISH_DOMAIN";

/// Nav host served from `platform/site`.
pub const NAV_HOST_ENV: &str = "FORGE_PUBLISH_NAV_HOST";

/// Deprecated: the target no longer hosts a deployment script tree.
/// Accepted so an existing environment does not fail, never used.
pub const SCRIPTS_ROOT_ENV: &str = "FORGE_PUBLISH_SCRIPTS_ROOT";

/// Default SSH host alias (the Mac).
pub const DEFAULT_SSH_TARGET: &str = "mac";

/// Default synced project tree on the Mac.
pub const DEFAULT_REMOTE_ROOT: &str = "/Users/allen/jenkins/projects";

/// Default per-project runtime tree on the Mac.
pub const DEFAULT_RUNTIME_ROOT: &str = "/Users/allen/jenkins/runtime";

/// Default router tree on the Mac.
pub const DEFAULT_PLATFORM_ROOT: &str = "/Users/allen/production/platform";

/// Default target-local secret tree on the Mac.
pub const DEFAULT_SECRETS_ROOT: &str = "/Users/allen/production/secrets";

/// Default shared-infrastructure tree on the Mac.
pub const DEFAULT_SHARED_INFRA_ROOT: &str = "/Users/allen/production/shared-infra";

/// Default public domain.
pub const DEFAULT_DOMAIN: &str = "tooosall.uk";

/// Default nav host.
pub const DEFAULT_NAV_HOST: &str = "apps";

/// Docker binary the Mac exposes.
pub const DOCKER_BIN: &str = "/usr/local/bin/docker";

/// PATH prepended for non-interactive SSH sessions, which omit
/// Docker Desktop's helper directory.
pub const DOCKER_PATH: &str =
    "/Applications/Docker.app/Contents/Resources/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";

/// Remote `mkdir` used to create runtime directories.
pub const REMOTE_MKDIR: &str = "/bin/mkdir";

/// Remote `mv` used for atomic artifact replacement.
pub const REMOTE_MV: &str = "/bin/mv";

/// Suffix every shipped artifact carries until it is moved into place.
pub const STAGED_SUFFIX: &str = ".forge-new";

/// Bounded size of the Compose config document collected from a target.
pub const MAX_COMPOSE_BYTES: usize = 4_096_000;

/// Bounded size of the `docker ps` port capture.
pub const MAX_DOCKER_PS_BYTES: usize = 262_144;

/// Compose file names probed on the target, in the sibling's order.
pub const COMPOSE_CANDIDATES: [&str; 3] = ["docker-compose.yml", "compose.yaml", "compose.yml"];
