//! Forge-owned remote-compose publish adapter
//! (`decoupled-remote-publish`).
//!
//! This adapter publishes a project to a generic Docker host that
//! carries **no deployment script tree**: the target needs Docker and
//! a file tree (`projects/<id>/`, `runtime/<id>/`, `platform/`) and
//! nothing else. Every stage is expressed as plain `ssh`, `rsync`,
//! `scp`, `mkdir`/`mv` and `docker compose` argument arrays issued
//! from Linux.
//!
//! | Stage | What Forge does |
//! |---|---|
//! | `Sync` | `ssh <t> mkdir -p` then `rsync -az` of the source tree |
//! | `Prepare` | read `RUNTIME_ROOT/port-registry.json` over SSH, allocate the 20-port block in Rust, render `ports.compose.yml` (and the shared-DB overlay) locally, ship both |
//! | `Db` | `docker compose -f shared-infra/compose.yml up -d production-postgres` plus a `production-db-network` check |
//! | `Deploy` | `docker compose -p forge-<id> … up -d --build`, then ship `platform/Caddyfile` + `site/index.html` and recreate the router |
//!
//! ## Port allocation and overlays are Forge-owned
//!
//! [`super::port_allocator`], [`super::db_overlay`] and [`super::caddy`]
//! carry the semantics the target scripts used to own. Forge collects
//! the inputs it needs from the target (`port-registry.json`, the
//! project Compose config, the ports Docker already binds) during
//! [`PublishAdapter::materialize`], computes every document locally,
//! and ships only finished files.
//!
//! ## Secrets stay on the target
//!
//! The adapter never reads `SECRETS_ROOT/<project>/.env` or
//! `.shared-db.env`. It probes only whether they exist (`ssh <t> test
//! -f <path>`) and passes them to Compose as repeated `--env-file`
//! arguments, which is the shell-free equivalent of the legacy
//! merge into `runtime/<project>/production.env`. Every captured
//! string passes [`crate::policy::redact_credentials`] before it can
//! reach a journal row or a rendered document.
//!
//! ## Migration
//!
//! [`super::jenkins::JenkinsAdapter`] (`ssh bash scripts/*.sh`) stays
//! compiled and selectable through [`ADAPTER_ENV`] so a real-deploy
//! failure rolls back with a flag, not a rebuild.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    Classification, CommandResult, CommandSpec, PublishAction, PublishAdapter, PublishRequest,
    SshTransport, StagePlan, PUBLISH_DEPLOY_TIMEOUT, PUBLISH_SYNC_TIMEOUT, STAGE_DB, STAGE_DEPLOY,
    STAGE_PREPARE, STAGE_SYNC,
};
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::publish::caddy;
use crate::publish::db_overlay;
use crate::publish::port_allocator::{
    self, PortAllocator, PortRange, PortRegistry, DEFAULT_BIND_ADDRESS,
};

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

/// Remote layout plus the public routing projection. Every root is an
/// environment override so a cloud migration is env-only; the
/// defaults preserve the current Mac layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteConfig {
    pub ssh_target: String,
    pub remote_root: String,
    pub runtime_root: String,
    pub platform_root: String,
    pub secrets_root: String,
    pub shared_infra_root: String,
    pub domain: String,
    pub nav_host: String,
}

/// Optional environment overrides; every field is `None` when the
/// variable is unset or blank.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Overrides {
    pub ssh_target: Option<String>,
    pub remote_root: Option<String>,
    pub runtime_root: Option<String>,
    pub platform_root: Option<String>,
    pub secrets_root: Option<String>,
    pub shared_infra_root: Option<String>,
    pub domain: Option<String>,
    pub nav_host: Option<String>,
}

impl Overrides {
    pub fn from_env() -> Self {
        fn read(key: &str) -> Option<String> {
            env::var(key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        }
        Overrides {
            ssh_target: read(SSH_TARGET_ENV),
            remote_root: read(REMOTE_ROOT_ENV),
            runtime_root: read(RUNTIME_ROOT_ENV),
            platform_root: read(PLATFORM_ROOT_ENV),
            secrets_root: read(SECRETS_ROOT_ENV),
            shared_infra_root: read(SHARED_INFRA_ROOT_ENV),
            domain: read(DOMAIN_ENV),
            nav_host: read(NAV_HOST_ENV),
        }
    }
}

impl RemoteConfig {
    pub fn from_env() -> Self {
        Self::from_overrides(&Overrides::from_env())
    }

    pub fn from_overrides(o: &Overrides) -> Self {
        RemoteConfig {
            ssh_target: o
                .ssh_target
                .clone()
                .unwrap_or_else(|| DEFAULT_SSH_TARGET.to_string()),
            remote_root: o
                .remote_root
                .clone()
                .unwrap_or_else(|| DEFAULT_REMOTE_ROOT.to_string()),
            runtime_root: o
                .runtime_root
                .clone()
                .unwrap_or_else(|| DEFAULT_RUNTIME_ROOT.to_string()),
            platform_root: o
                .platform_root
                .clone()
                .unwrap_or_else(|| DEFAULT_PLATFORM_ROOT.to_string()),
            secrets_root: o
                .secrets_root
                .clone()
                .unwrap_or_else(|| DEFAULT_SECRETS_ROOT.to_string()),
            shared_infra_root: o
                .shared_infra_root
                .clone()
                .unwrap_or_else(|| DEFAULT_SHARED_INFRA_ROOT.to_string()),
            domain: o
                .domain
                .clone()
                .unwrap_or_else(|| DEFAULT_DOMAIN.to_string()),
            nav_host: o
                .nav_host
                .clone()
                .unwrap_or_else(|| DEFAULT_NAV_HOST.to_string()),
        }
    }

    /// Every root must be an absolute, traversal-free path so a
    /// misconfigured cloud environment cannot compose a command that
    /// writes outside the declared tree.
    pub fn validate(&self) -> Result<(), ForgeError> {
        for (name, value) in [
            ("remote root", &self.remote_root),
            ("runtime root", &self.runtime_root),
            ("platform root", &self.platform_root),
            ("secrets root", &self.secrets_root),
            ("shared infra root", &self.shared_infra_root),
        ] {
            if !value.starts_with('/') {
                return Err(ForgeError::PublishInvalid {
                    reason: format!("publish {name} `{value}` must be an absolute path"),
                });
            }
            if value.split('/').any(|segment| segment == "..") {
                return Err(ForgeError::PublishInvalid {
                    reason: format!("publish {name} `{value}` must not contain `..` segments"),
                });
            }
        }
        if self.ssh_target.trim().is_empty() {
            return Err(ForgeError::PublishInvalid {
                reason: format!("publish ssh target from {SSH_TARGET_ENV} must not be empty"),
            });
        }
        if self.domain.trim().is_empty() || self.nav_host.trim().is_empty() {
            return Err(ForgeError::PublishInvalid {
                reason: format!("publish {DOMAIN_ENV} and {NAV_HOST_ENV} must both name a host"),
            });
        }
        Ok(())
    }

    pub fn remote_project_dir(&self, project_id: &str) -> String {
        format!("{}/{}", self.remote_root.trim_end_matches('/'), project_id)
    }

    pub fn remote_runtime_dir(&self, project_id: &str) -> String {
        format!("{}/{}", self.runtime_root.trim_end_matches('/'), project_id)
    }

    pub fn registry_path(&self) -> String {
        format!(
            "{}/port-registry.json",
            self.runtime_root.trim_end_matches('/')
        )
    }

    pub fn project_env_path(&self, project_id: &str) -> String {
        format!(
            "{}/{}/.env",
            self.secrets_root.trim_end_matches('/'),
            project_id
        )
    }

    pub fn shared_db_env_path(&self, project_id: &str) -> String {
        format!(
            "{}/{}/.shared-db.env",
            self.secrets_root.trim_end_matches('/'),
            project_id
        )
    }
}

/// One locally rendered document plus its destination on the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedArtifact {
    pub local: PathBuf,
    pub remote: String,
}

/// Everything [`RemoteComposeAdapter`] computed before planning:
/// the target facts it observed and the documents it rendered.
#[derive(Debug, Default)]
struct RemoteState {
    registry: PortRegistry,
    compose_file: Option<String>,
    compose: Value,
    unavailable: BTreeSet<u32>,
    shared_db_env: bool,
    project_env: bool,
    /// True when a shared-database overlay was actually rendered and
    /// shipped. False when the project runs standalone databases
    /// (deployed as-is) or declares no shared-DB intent.
    shared_db_overlay: bool,
    /// Compose profiles declared by the project's own Compose file.
    /// A bare `up` selects no profile-gated service (`no service
    /// selected`), so deploys — and the `config` read that feeds
    /// port allocation — enable every declared profile: Forge
    /// publishes the whole project, and the legacy lane never
    /// selected profiles either.
    profiles: Vec<String>,
    scale_args: Vec<String>,
    artifacts: Vec<StagedArtifact>,
    notes: Vec<String>,
}

/// The decoupled publish adapter. State is interior-mutable because
/// the orchestrator holds the adapter behind `&dyn PublishAdapter` and
/// calls [`PublishAdapter::materialize`] before planning.
pub struct RemoteComposeAdapter {
    config: RemoteConfig,
    staging_root: PathBuf,
    state: RefCell<RemoteState>,
}

impl RemoteComposeAdapter {
    pub fn new(config: RemoteConfig) -> Self {
        RemoteComposeAdapter {
            staging_root: env::temp_dir().join("forge-publish"),
            config,
            state: RefCell::new(RemoteState::default()),
        }
    }

    pub fn from_env() -> Self {
        RemoteComposeAdapter::new(RemoteConfig::from_env())
    }

    /// Point rendered documents at a caller-owned directory instead of
    /// the shared temporary root. Tests use this; production does not.
    pub fn with_staging_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.staging_root = root.into();
        self
    }

    pub fn config(&self) -> &RemoteConfig {
        &self.config
    }

    /// The documents the last [`PublishAdapter::materialize`] run
    /// rendered, for evidence and tests.
    pub fn staged_artifacts(&self) -> Vec<StagedArtifact> {
        self.state.borrow().artifacts.clone()
    }

    /// Human-readable notes about what the adapter observed and
    /// computed, already redacted.
    pub fn notes(&self) -> Vec<String> {
        self.state.borrow().notes.clone()
    }

    fn remote_project_dir(&self, project_id: &str) -> String {
        self.config.remote_project_dir(project_id)
    }

    fn compose_file(&self) -> Result<String, ForgeError> {
        self.state.borrow().compose_file.clone().ok_or_else(|| {
            ForgeError::DeployTargetUnavailable {
                reason: "publish did not resolve a Compose file for this project; \
                         run the prepare stage before deploying"
                    .to_string(),
            }
        })
    }

    /// Ship one locally rendered document: `scp` to a staged name on
    /// the target, then an atomic rename into place. Nothing is ever
    /// written outside the destination tree and a half-copied document
    /// is never visible to the target's Compose.
    fn ship(&self, local: &Path, remote: &str) -> Vec<CommandSpec> {
        vec![
            CommandSpec::new("scp", "ship rendered document")
                .arg(local)
                .arg(format!(
                    "{}:{remote}{STAGED_SUFFIX}",
                    self.config.ssh_target
                )),
            CommandSpec::new("ssh", "publish rendered document")
                .arg(self.config.ssh_target.as_str())
                .arg(REMOTE_MV)
                .arg("-f")
                .arg(format!("{remote}{STAGED_SUFFIX}"))
                .arg(remote),
        ]
    }
}

impl PublishAdapter for RemoteComposeAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn label(&self) -> &'static str {
        "Remote compose (generic Docker host)"
    }

    fn materialize(
        &self,
        request: &PublishRequest,
        transport: &dyn SshTransport,
        dry_run: bool,
    ) -> Result<(), ForgeError> {
        if !request.action.stages().contains(&PublishAction::Prepare) {
            return Ok(());
        }
        self.config.validate()?;
        let project_id = request.project_id.clone();
        let mut state = RemoteState {
            compose_file: local_compose_file(&request.project_dir),
            ..RemoteState::default()
        };
        // Profiles come from the local checkout (the exact tree Sync
        // ships), so both dry-run previews and real runs render the
        // same `--profile` selection without another target round
        // trip. Absent or unreadable means no profiles, as before.
        if let Some(file) = state.compose_file.clone() {
            state.profiles = compose_profiles(&request.project_dir, &file);
            if !state.profiles.is_empty() {
                state.notes.push(format!(
                    "compose declares profile(s) {}; deploys enable all of them",
                    state.profiles.join(", ")
                ));
            }
        }

        if !dry_run {
            state.registry = self.read_registry(transport, &mut state)?;
            if state.compose_file.is_none() {
                match self.resolve_compose_file(transport, &project_id) {
                    Ok(candidate) => state.compose_file = Some(candidate),
                    Err(error) => {
                        return Err(ForgeError::DeployTargetUnavailable {
                            reason: format!(
                                "{error}; no local Compose file either (looked for {})",
                                COMPOSE_CANDIDATES.join(", ")
                            ),
                        });
                    }
                }
            }
            state.compose =
                self.read_compose_config(transport, &mut state, &project_id, &request.project_dir)?;
            state.unavailable = self.read_unavailable_ports(transport, &mut state)?;
            state.shared_db_env =
                self.probe_target_file(transport, &self.config.shared_db_env_path(&project_id));
            state.project_env =
                self.probe_target_file(transport, &self.config.project_env_path(&project_id));
        }
        state.notes.push(if dry_run {
            "dry-run: target registry and Compose config were not read; \
             the rendered documents preview an empty registry"
                .to_string()
        } else {
            format!(
                "target registry {} carries {} project(s)",
                self.config.registry_path(),
                state.registry.projects.len()
            )
        });

        self.render_artifacts(&mut state, &project_id, dry_run)?;
        self.state.replace(state);
        Ok(())
    }

    fn plan(
        &self,
        request: &PublishRequest,
        stage: PublishAction,
    ) -> Result<StagePlan, ForgeError> {
        match stage {
            PublishAction::Sync => Ok(self.plan_sync(request)?),
            PublishAction::Db => Ok(self.plan_db(request)?),
            PublishAction::Prepare => Ok(self.plan_prepare(request)?),
            PublishAction::Deploy => Ok(self.plan_deploy(request)?),
            PublishAction::All => Err(ForgeError::PublishInvalid {
                reason: "PublishAction::All cannot be planned; it expands via the orchestrator"
                    .to_string(),
            }),
        }
    }

    fn classify(&self, plan: &StagePlan, result: &CommandResult) -> Classification {
        match plan.stage.as_str() {
            STAGE_SYNC => classify_sync(result, &self.config),
            STAGE_DB => classify_db(result),
            STAGE_PREPARE => classify_prepare(result, &self.config),
            STAGE_DEPLOY => classify_deploy(result, &self.config),
            _ => classify_generic(result),
        }
    }

    fn subdomain(&self, project_id: &str) -> Option<String> {
        Some(format!("{project_id}.{}", self.config.domain))
    }
}

// ---------------------------------------------------------------------------
// Stage plans
// ---------------------------------------------------------------------------

impl RemoteComposeAdapter {
    fn plan_sync(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        validate_project_id(&request.project_id)?;
        let remote_dir = self.remote_project_dir(&request.project_id);
        let local = format!(
            "{}/",
            request
                .project_dir
                .display()
                .to_string()
                .trim_end_matches('/')
        );
        let mkdir = CommandSpec::new("ssh", "remote project directory")
            .arg(self.config.ssh_target.as_str())
            .arg(REMOTE_MKDIR)
            .arg("-p")
            .arg(&remote_dir);
        // `data/` holds mounted volume state (often root-owned on
        // the dev host, e.g. database files): it is runtime state,
        // never source, and syncing it fails closed with rsync
        // exit 23. The legacy lane carries the same unreadable tree
        // and fails identically, so excluding it here strictly
        // advances the decoupled lane without hiding source.
        let mut exclusions: Vec<String> = Vec::new();
        for exclusion in [
            ".git/",
            "node_modules/",
            "target/",
            "dist/",
            "build/",
            "data/",
        ] {
            exclusions.push("--exclude".to_string());
            exclusions.push(exclusion.to_string());
        }
        // Source syncs carry the sync ceiling: multi-GB trees
        // under parallel-fleet contention routinely exceed the
        // interactive 60s transport default (`fleet-live-rollout`).
        let rsync = CommandSpec::new("rsync", "project source sync")
            .with_args(["-az", "--human-readable"])
            .with_args(exclusions)
            .with_args([
                "-e".to_string(),
                "ssh".to_string(),
                local.clone(),
                format!("{}:{remote_dir}/", self.config.ssh_target),
            ])
            .with_timeout(PUBLISH_SYNC_TIMEOUT);
        Ok(StagePlan {
            stage: STAGE_SYNC.to_string(),
            commands: vec![mkdir, rsync],
            project_id: request.project_id.clone(),
        })
    }

    fn plan_prepare(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        validate_project_id(&request.project_id)?;
        let runtime_dir = self.config.remote_runtime_dir(&request.project_id);
        let mut commands: Vec<CommandSpec> = vec![CommandSpec::new("ssh", "runtime directory")
            .arg(self.config.ssh_target.as_str())
            .arg(REMOTE_MKDIR)
            .arg("-p")
            .arg(&runtime_dir)];
        let artifacts = self.staged_artifacts();
        if artifacts.is_empty() {
            return Err(ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "publish prepare rendered no documents for `{}`; the port registry could not be read",
                    request.project_id
                ),
            });
        }
        for artifact in artifacts {
            commands.extend(self.ship(&artifact.local, &artifact.remote));
        }
        Ok(StagePlan {
            stage: STAGE_PREPARE.to_string(),
            commands,
            project_id: request.project_id.clone(),
        })
    }

    fn plan_db(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        let shared_infra = format!(
            "{}/compose.yml",
            self.config.shared_infra_root.trim_end_matches('/')
        );
        let up = docker_command(&self.config, "shared PostgreSQL").with_args([
            "compose",
            "-f",
            shared_infra.as_str(),
            "-p",
            "shared-postgres",
            "up",
            "-d",
            "production-postgres",
        ]);
        let inspect = docker_command(&self.config, "shared database network").with_args([
            "network",
            "inspect",
            db_overlay::SHARED_DB_NETWORK,
        ]);
        Ok(StagePlan {
            stage: STAGE_DB.to_string(),
            commands: vec![up, inspect],
            project_id: request.project_id.clone(),
        })
    }

    fn plan_deploy(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        validate_project_id(&request.project_id)?;
        let project_id = request.project_id.as_str();
        let remote_dir = self.remote_project_dir(project_id);
        let runtime_dir = self.config.remote_runtime_dir(project_id);
        let compose_file = self.compose_file()?;
        let shared_db = self.state.borrow().shared_db_overlay;

        let mut argv: Vec<String> = vec!["compose".to_string()];
        argv.extend(profile_args(&self.state.borrow().profiles));
        for env_file in self.deploy_env_files(project_id) {
            argv.push("--env-file".to_string());
            argv.push(env_file);
        }
        argv.extend([
            "-p".to_string(),
            compose_project_identity(project_id),
            "--project-directory".to_string(),
            remote_dir.clone(),
            "-f".to_string(),
            format!("{remote_dir}/{compose_file}"),
            "-f".to_string(),
            format!("{runtime_dir}/ports.compose.yml"),
        ]);
        if shared_db {
            argv.push("-f".to_string());
            argv.push(format!("{runtime_dir}/shared-db.compose.yml"));
        }
        argv.extend([
            "up".to_string(),
            "-d".to_string(),
            "--build".to_string(),
            "--remove-orphans".to_string(),
        ]);
        let scale_args = self.state.borrow().scale_args.clone();
        argv.extend(scale_args);
        // Container builds routinely exceed the interactive subprocess
        // bound; the deploy stage carries the long build ceiling.
        let deploy = docker_command(&self.config, "project deploy")
            .with_args(argv)
            .with_timeout(PUBLISH_DEPLOY_TIMEOUT);

        let mut commands = vec![deploy];
        let router_artifacts: Vec<StagedArtifact> = self
            .staged_artifacts()
            .into_iter()
            .filter(|artifact| artifact.remote.starts_with(&self.config.platform_root))
            .collect();
        if router_artifacts.is_empty() {
            return Err(ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "publish deploy rendered no router document for `{project_id}`; \
                     run the prepare stage before deploying"
                ),
            });
        }
        for artifact in router_artifacts {
            commands.extend(self.ship(&artifact.local, &artifact.remote));
        }
        commands.push(
            docker_command(&self.config, "router reload")
                .with_args([
                    "compose",
                    "-f",
                    &format!(
                        "{}/compose.yml",
                        self.config.platform_root.trim_end_matches('/')
                    ),
                    "up",
                    "-d",
                    "--force-recreate",
                ])
                .with_timeout(PUBLISH_DEPLOY_TIMEOUT),
        );
        Ok(StagePlan {
            stage: STAGE_DEPLOY.to_string(),
            commands,
            project_id: request.project_id.clone(),
        })
    }

    /// Target-local env files referenced by the deploy, in Compose's
    /// own precedence order. Their contents never reach Linux.
    fn deploy_env_files(&self, project_id: &str) -> Vec<String> {
        let state = self.state.borrow();
        let mut files = Vec::new();
        if state.project_env {
            files.push(self.config.project_env_path(project_id));
        }
        if state.shared_db_env {
            files.push(self.config.shared_db_env_path(project_id));
        }
        files
    }
}

/// The Compose project name a decoupled deploy owns. Distinct from
/// the retained legacy `jenkins-<project>` identity so a rollback is
/// visible in `docker ps` rather than silently reusing containers.
pub fn compose_project_identity(project_id: &str) -> String {
    format!("forge-{project_id}")
}

// ---------------------------------------------------------------------------
// Materialisation: read the target, render locally
// ---------------------------------------------------------------------------

impl RemoteComposeAdapter {
    fn read_registry(
        &self,
        transport: &dyn SshTransport,
        state: &mut RemoteState,
    ) -> Result<PortRegistry, ForgeError> {
        let path = self.config.registry_path();
        let command = CommandSpec::new("ssh", "read target port registry")
            .arg(self.config.ssh_target.as_str())
            .arg("cat")
            .arg(&path);
        match transport.run(command) {
            Ok(result) if result.success() => {
                if result.stdout.trim().is_empty() {
                    state.notes.push(format!(
                        "target registry {path} is empty; starting a new registry"
                    ));
                    return Ok(PortRegistry::default());
                }
                let registry = PortRegistry::parse(result.stdout.as_bytes())?;
                for identity in registry.projects.keys() {
                    validate_project_id(&caddy::project_id(identity)).map_err(|_| {
                        ForgeError::DeployTargetUnavailable {
                            reason: format!(
                                "target port registry {path} carries unsafe identity `{identity}`"
                            ),
                        }
                    })?;
                }
                Ok(registry)
            }
            Ok(result) => {
                let stderr = redact_credentials(&result.stderr);
                if result.stdout.trim().is_empty() && stderr.to_lowercase().contains("no such file")
                {
                    // First contact with a fresh target (or a fresh
                    // cloud host): there is no registry to converge
                    // with yet, so bootstrap an empty one locally and
                    // ship it in Prepare. Any other read failure stays
                    // a typed target error.
                    state.notes.push(format!(
                        "target registry {path} is absent; bootstrapping a new registry"
                    ));
                    return Ok(PortRegistry::default());
                }
                Err(ForgeError::DeployTargetUnavailable {
                    reason: format!(
                        "cannot read target port registry {path} (exit {}): {}",
                        result.status,
                        stderr.trim()
                    ),
                })
            }
            Err(error) => Err(ForgeError::DeployTargetUnavailable {
                reason: format!("cannot reach `{path}` on the target: {error}"),
            }),
        }
    }

    fn resolve_compose_file(
        &self,
        transport: &dyn SshTransport,
        project_id: &str,
    ) -> Result<String, ForgeError> {
        let dir = self.remote_project_dir(project_id);
        for candidate in COMPOSE_CANDIDATES {
            let path = format!("{dir}/{candidate}");
            let command = CommandSpec::new("ssh", "probe Compose file")
                .arg(self.config.ssh_target.as_str())
                .arg("test")
                .arg("-f")
                .arg(&path);
            if let Ok(result) = transport.run(command) {
                if result.success() {
                    return Ok(candidate.to_string());
                }
            }
        }
        Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "no primary Compose file found in {dir}; looked for {}",
                COMPOSE_CANDIDATES.join(", ")
            ),
        })
    }

    fn read_compose_config(
        &self,
        transport: &dyn SshTransport,
        state: &mut RemoteState,
        project_id: &str,
        project_dir: &Path,
    ) -> Result<Value, ForgeError> {
        let compose_file = state.compose_file.clone().unwrap_or_default();
        let path = format!("{}/{compose_file}", self.remote_project_dir(project_id));
        let mut argv: Vec<String> = vec!["compose".to_string()];
        argv.extend(profile_args(&state.profiles));
        argv.extend(["-f".to_string(), path.clone()]);
        argv.extend(["config", "--no-interpolate", "--format", "json"].map(str::to_string));
        let command = docker_command(&self.config, "read Compose config").with_args(argv);
        let result =
            transport
                .run(command)
                .map_err(|error| ForgeError::DeployTargetUnavailable {
                    reason: format!(
                        "cannot run `docker compose config` for `{project_id}`: {error}"
                    ),
                })?;
        if result.success() {
            if result.stdout.len() > MAX_COMPOSE_BYTES {
                return Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "`docker compose config` for `{project_id}` returned {} bytes, above the {MAX_COMPOSE_BYTES} byte bound",
                        result.stdout.len()
                    ),
                });
            }
            return serde_json::from_str(&result.stdout).map_err(|error| {
                ForgeError::DeployTargetUnavailable {
                    reason: format!(
                        "`docker compose config` for `{project_id}` is not JSON: {error}"
                    ),
                }
            });
        }
        // The target has not been synced yet (first decoupled run for
        // this project): fall back to the local checkout that Sync is
        // about to ship, so a never-synced project does not fail
        // before its first Sync. Any other target failure stays typed.
        let stderr = redact_credentials(&result.stderr);
        if stderr.to_lowercase().contains("no such file") && !compose_file.is_empty() {
            if let Some(document) = read_local_compose(project_dir, &compose_file)? {
                state.notes.push(format!(
                    "target Compose file {path} is absent; used the local checkout for port and overlay rendering"
                ));
                return Ok(document);
            }
        }
        Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "`docker compose config` failed for {path} (exit {}): {}",
                result.status,
                stderr.trim()
            ),
        })
    }

    /// Host ports the target already binds, discovered from one
    /// `docker ps` capture. This replaces the sibling's per-port bind
    /// probe without introducing a target script.
    fn read_unavailable_ports(
        &self,
        transport: &dyn SshTransport,
        state: &mut RemoteState,
    ) -> Result<BTreeSet<u32>, ForgeError> {
        let command = docker_command(&self.config, "read bound host ports").with_args([
            "ps",
            "--format",
            "{{.Names}}|{{.Ports}}",
        ]);
        let result =
            transport
                .run(command)
                .map_err(|error| ForgeError::DeployTargetUnavailable {
                    reason: format!("cannot list target containers: {error}"),
                })?;
        if !result.success() {
            state.notes.push(format!(
                "target container list unavailable (exit {}); allocating from the registry only",
                result.status
            ));
            return Ok(BTreeSet::new());
        }
        if result.stdout.len() > MAX_DOCKER_PS_BYTES {
            return Ok(BTreeSet::new());
        }
        let range = PortRange::default();
        Ok(
            port_allocator::published_ports_from_docker_ps(&result.stdout)
                .into_iter()
                .filter(|port| *port >= range.start && *port <= range.end)
                .collect(),
        )
    }

    /// Probe a target-local secret path by name only. The contents
    /// are never fetched, so the Mac-local secret boundary holds.
    fn probe_target_file(&self, transport: &dyn SshTransport, path: &str) -> bool {
        let command = CommandSpec::new("ssh", "probe target-local file")
            .arg(self.config.ssh_target.as_str())
            .arg("test")
            .arg("-f")
            .arg(path);
        matches!(transport.run(command), Ok(result) if result.success())
    }

    fn render_artifacts(
        &self,
        state: &mut RemoteState,
        project_id: &str,
        dry_run: bool,
    ) -> Result<(), ForgeError> {
        let identity = format!("local:{project_id}");
        let ports = port_allocator::extract_published_ports(&state.compose);
        let legacy: BTreeMap<String, u32> = ports
            .iter()
            .map(|port| (port.key.clone(), port.published))
            .collect();
        let services: Vec<String> = ports.iter().map(|port| port.key.clone()).collect();
        let allocator = PortAllocator::new(PortRange::default());
        let record = allocator.allocate(
            &mut state.registry,
            &identity,
            &services,
            &legacy,
            &state.unavailable,
        )?;
        state.notes.push(format!(
            "allocated block {}-{} for `{identity}` ({} published port(s))",
            record.base,
            record.base + port_allocator::DEFAULT_BLOCK_SIZE - 1,
            record.services.len()
        ));

        let ports_override = if ports.is_empty() {
            port_allocator::render_empty_override()
        } else {
            port_allocator::render_override(&ports, &record.services, DEFAULT_BIND_ADDRESS)?
        };

        let runtime_dir = self.config.remote_runtime_dir(project_id);
        let mut artifacts: Vec<(String, String)> = vec![(
            self.config.registry_path(),
            port_allocator::PortRegistry::render(&state.registry)?,
        )];
        artifacts.push((format!("{runtime_dir}/ports.compose.yml"), ports_override));
        let databases = db_overlay::database_services(&state.compose);
        let applications = db_overlay::application_services(&state.compose);
        if applications.is_empty() && !databases.is_empty() {
            // Standalone database compose (e.g. an infra-only
            // project running its own postgres): no application
            // service consumes the shared database, so there is
            // nothing to remap and scaling the local databases to
            // zero would destroy the project's data source.
            // Deploy the compose as-is without an overlay.
            state.notes.push(format!(
                "project `{}` runs standalone database service(s) {}; deployed without a shared-database overlay",
                project_id,
                databases.join(", ")
            ));
        } else if state.shared_db_env || state.project_env {
            let env_path = if state.project_env {
                Some(self.config.project_env_path(project_id))
            } else {
                None
            };
            let overlay = db_overlay::render_overlay(
                &state.compose,
                project_id,
                env_path.as_deref(),
                state.shared_db_env,
            )?;
            state.scale_args = overlay.scale_args.clone();
            state.shared_db_overlay = true;
            state.notes.push(format!(
                "rendered shared-database overlay for `{}` ({} service(s) scaled off)",
                project_id,
                state.scale_args.len() / 2
            ));
            artifacts.push((format!("{runtime_dir}/shared-db.compose.yml"), overlay.yaml));
        }
        let platform_root = self.config.platform_root.trim_end_matches('/');
        artifacts.push((
            format!("{platform_root}/Caddyfile"),
            caddy::render_caddyfile(&state.registry, &self.config.domain, &self.config.nav_host)?,
        ));
        artifacts.push((
            format!("{platform_root}/site/index.html"),
            caddy::render_index(&state.registry, &self.config.domain)?,
        ));

        let staging_dir = self.staging_root.join(project_id);
        state.notes.push(format!(
            "rendered {} document(s) into {}",
            artifacts.len(),
            staging_dir.display()
        ));
        for (name, contents) in &artifacts {
            let destination = staging_dir.join(file_name(name));
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!(
                        "cannot create publish staging directory {}: {error}",
                        parent.display()
                    ),
                })?;
            }
            std::fs::write(&destination, contents).map_err(|error| ForgeError::PublishInvalid {
                reason: format!(
                    "cannot write publish document {}: {error}",
                    destination.display()
                ),
            })?;
            state.artifacts.push(StagedArtifact {
                local: destination,
                remote: name.clone(),
            });
        }
        if dry_run {
            state.notes.push(
                "dry-run: documents are a local preview only; no subprocess was spawned and the target is unchanged"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The Compose file a local project checkout ships, when present.
fn local_compose_file(project_dir: &Path) -> Option<String> {
    COMPOSE_CANDIDATES
        .iter()
        .find(|candidate| project_dir.join(candidate).is_file())
        .map(|candidate| (*candidate).to_string())
}

/// Compose profiles declared by the local checkout's Compose file,
/// sorted and deduplicated. Best-effort: any read or parse failure
/// means no profiles (the previous behavior), never a refusal.
fn compose_profiles(project_dir: &Path, compose_file: &str) -> Vec<String> {
    let path = project_dir.join(compose_file);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut profiles = BTreeSet::new();
    if let Some(services) = yaml.get("services").and_then(|v| v.as_mapping()) {
        for (_, service) in services {
            if let Some(list) = service.get("profiles").and_then(|v| v.as_sequence()) {
                for entry in list {
                    if let Some(name) = entry.as_str() {
                        let name = name.trim();
                        if !name.is_empty() {
                            profiles.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }
    profiles.into_iter().collect()
}

/// `--profile` argv for the given profiles, in compose flag position
/// (directly after `compose`, before `-f`).
fn profile_args(profiles: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(profiles.len() * 2);
    for profile in profiles {
        argv.push("--profile".to_string());
        argv.push(profile.clone());
    }
    argv
}
/// Parse the local checkout's Compose file into the same JSON shape
/// `docker compose config --format json` produces on the target.
/// Used only when the target has no Compose file yet (first sync):
/// returns `Ok(None)` when the local file is absent so the caller
/// keeps its original target error, and refuses an unreadable or
/// unparseable local document instead of deploying blind.
fn read_local_compose(project_dir: &Path, compose_file: &str) -> Result<Option<Value>, ForgeError> {
    let path = project_dir.join(compose_file);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot read local Compose file {}: {error}", path.display()),
    })?;
    if bytes.len() > MAX_COMPOSE_BYTES {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} is {} bytes, above the {MAX_COMPOSE_BYTES} byte bound",
                path.display(),
                bytes.len()
            ),
        });
    }
    let text = String::from_utf8(bytes).map_err(|_| ForgeError::PublishInvalid {
        reason: format!("local Compose file {} is not valid UTF-8", path.display()),
    })?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} is not valid YAML: {error}",
                path.display()
            ),
        })?;
    serde_json::to_value(yaml)
        .map(Some)
        .map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} does not project to JSON: {error}",
                path.display()
            ),
        })
}

fn validate_project_id(project_id: &str) -> Result<(), ForgeError> {
    if project_id.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if !project_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "project id `{project_id}` must be kebab/snake-case (letters, digits, dash, underscore)"
            ),
        });
    }
    Ok(())
}

/// `ssh <target> env PATH=… BUILDKIT_PROGRESS=plain <docker> …`. The
/// remote `env` keeps the command free of any shell script while
/// giving Docker Desktop's helper directory to non-interactive SSH
/// sessions and plain build output to BuildKit.
fn docker_command(config: &RemoteConfig, label: &str) -> CommandSpec {
    let mut spec = CommandSpec::new("ssh", label);
    spec.args.push(OsString::from(config.ssh_target.as_str()));
    spec.args.push(OsString::from("env"));
    spec.args
        .push(OsString::from(format!("PATH={DOCKER_PATH}")));
    spec.args.push(OsString::from("BUILDKIT_PROGRESS=plain"));
    spec.args.push(OsString::from(DOCKER_BIN));
    spec
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

fn classify_sync(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("project source tree synced to the target");
    }
    let recovery = match result.status {
        255 | 12 => vec![
            format!(
                "verify the SSH alias `{}` resolves in ~/.ssh/config",
                config.ssh_target
            ),
            "partial state may exist on the target; rerun `forge publish sync` to repair"
                .to_string(),
        ],
        _ => vec!["inspect the ssh/rsync output above for the underlying cause".to_string()],
    };
    Classification::failed("sync failed", recovery)
}

fn classify_prepare(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("host ports allocated and runtime documents shipped");
    }
    Classification::failed(
        "publish prepare failed",
        vec![
            format!("verify {} is writable on the target", config.runtime_root),
            "verify the target is reachable and the port registry parses".to_string(),
        ],
    )
}

fn classify_db(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("shared PostgreSQL is running on the target");
    }
    let recovery = match result.status {
        1 => vec![
            format!(
                "verify Docker is running on the target and {}/compose.yml is present",
                "shared-infra"
            ),
            format!(
                "create the network with `docker network create {}`",
                db_overlay::SHARED_DB_NETWORK
            ),
        ],
        _ => vec![
            "verify the shared-infrastructure Compose file and its admin env file".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("shared PostgreSQL is unavailable", recovery)
}

fn classify_deploy(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("compose project started on the target");
    }
    let recovery = match result.status {
        1 => vec![
            "run `forge publish prepare <project>` first to allocate ports and ship the runtime documents"
                .to_string(),
            format!(
                "verify {}/<project>/ports.compose.yml exists on the target",
                config.runtime_root
            ),
        ],
        _ => vec![
            "review the target compose output above for the failing service".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("compose deploy rejected the request", recovery)
}

fn classify_generic(result: &CommandResult) -> Classification {
    if result.success() {
        Classification::done("stage completed")
    } else {
        Classification::failed(
            "stage failed",
            vec![format!("inspect exit {}", result.status)],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::{RecordingTransport, STATUS_DONE};

    fn config() -> RemoteConfig {
        RemoteConfig::from_overrides(&Overrides::default())
    }

    fn adapter() -> RemoteComposeAdapter {
        RemoteComposeAdapter::new(config())
    }

    fn request(project_id: &str, dir: &Path, action: PublishAction) -> PublishRequest {
        PublishRequest {
            project_id: project_id.to_string(),
            project_dir: dir.to_path_buf(),
            action,
            dry_run: true,
        }
    }

    fn local_project() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"), "services: {}\n").unwrap();
        dir
    }

    #[test]
    fn config_defaults_preserve_the_mac_layout() {
        let cfg = config();
        assert_eq!(cfg.ssh_target, "mac");
        assert_eq!(cfg.remote_root, "/Users/allen/jenkins/projects");
        assert_eq!(cfg.runtime_root, "/Users/allen/jenkins/runtime");
        assert_eq!(cfg.platform_root, "/Users/allen/production/platform");
        assert_eq!(cfg.secrets_root, "/Users/allen/production/secrets");
        assert_eq!(
            cfg.shared_infra_root,
            "/Users/allen/production/shared-infra"
        );
        assert_eq!(cfg.domain, "tooosall.uk");
        assert_eq!(cfg.nav_host, "apps");
        cfg.validate().unwrap();
    }

    #[test]
    fn cloud_roots_are_env_only_and_validate() {
        let cfg = RemoteConfig::from_overrides(&Overrides {
            ssh_target: Some("cloud".to_string()),
            remote_root: Some("/srv/projects".to_string()),
            runtime_root: Some("/srv/runtime".to_string()),
            platform_root: Some("/srv/platform".to_string()),
            secrets_root: Some("/srv/secrets".to_string()),
            shared_infra_root: Some("/srv/shared-infra".to_string()),
            domain: Some("tooosall.uk".to_string()),
            nav_host: Some("apps".to_string()),
        });
        cfg.validate().unwrap();
        assert_eq!(cfg.remote_project_dir("alethefy"), "/srv/projects/alethefy");
        assert_eq!(cfg.remote_runtime_dir("alethefy"), "/srv/runtime/alethefy");
        assert_eq!(cfg.registry_path(), "/srv/runtime/port-registry.json");
    }

    #[test]
    fn relative_or_traversing_roots_are_refused() {
        let cfg = RemoteConfig {
            remote_root: "relative/projects".to_string(),
            ..config()
        };
        assert_eq!(cfg.validate().unwrap_err().code(), "publish-invalid");
        let cfg = RemoteConfig {
            secrets_root: "/srv/../etc".to_string(),
            ..config()
        };
        assert_eq!(cfg.validate().unwrap_err().code(), "publish-invalid");
    }

    #[test]
    fn adapter_id_and_label_are_stable() {
        let adapter = adapter();
        assert_eq!(adapter.id(), "remote-compose");
        assert_eq!(adapter.label(), "Remote compose (generic Docker host)");
        assert_eq!(
            adapter.subdomain("demo"),
            Some("demo.tooosall.uk".to_string())
        );
    }

    #[test]
    fn plan_never_references_a_target_script() {
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        for stage in PublishAction::All.stages() {
            let plan = adapter.plan(&req, stage.clone()).unwrap();
            let rendered = plan.render();
            for forbidden in [
                "project-ports.sh",
                "project-action.sh",
                "shared-postgres.sh",
                "bash",
            ] {
                assert!(
                    !rendered.contains(forbidden),
                    "stage {} rendered `{forbidden}`: {rendered}",
                    stage.label()
                );
            }
            assert!(rendered.contains("ssh ") || rendered.contains("rsync "));
        }
    }

    #[test]
    fn sync_plan_is_mkdir_plus_rsync() {
        let dir = local_project();
        let plan = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Sync),
                PublishAction::Sync,
            )
            .unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("ssh mac /bin/mkdir -p /Users/allen/jenkins/projects/alethefy"));
        assert!(rendered.contains("rsync -az --human-readable --exclude .git/"));
        assert!(rendered.contains("mac:/Users/allen/jenkins/projects/alethefy/"));
    }

    #[test]
    fn db_plan_starts_shared_postgres_without_a_script() {
        let dir = local_project();
        let plan = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Db),
                PublishAction::Db,
            )
            .unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("/usr/local/bin/docker compose -f /Users/allen/production/shared-infra/compose.yml -p shared-postgres up -d production-postgres"));
        assert!(rendered.contains("network inspect production-db-network"));
        assert!(rendered.contains("PATH=/Applications/Docker.app/Contents/Resources/bin"));
        assert!(rendered.contains("BUILDKIT_PROGRESS=plain"));
    }

    #[test]
    fn declared_profiles_are_enabled_on_deploy() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    image: demo:latest\n    profiles: [base, app]\n  db:\n    image: postgres:16\n    profiles:\n      - base\n",
        )
        .unwrap();
        assert_eq!(
            compose_profiles(dir.path(), "docker-compose.yml"),
            vec!["app".to_string(), "base".to_string()]
        );
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("profiled", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("profile(s) app, base")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(
            rendered.contains("--profile app --profile base"),
            "deploy was: {rendered}"
        );
    }

    #[test]
    fn missing_or_profileless_compose_means_no_profiles() {
        let dir = local_project();
        assert!(compose_profiles(dir.path(), "docker-compose.yml").is_empty());
        assert!(compose_profiles(dir.path(), "compose.yaml").is_empty());
        assert!(profile_args(&[]).is_empty());
        assert_eq!(
            profile_args(&["app".to_string()]),
            vec!["--profile".to_string(), "app".to_string()]
        );
    }

    #[test]
    fn deploy_without_prepare_refuses_as_target_unavailable() {
        let dir = local_project();
        let err = adapter()
            .plan(
                &request("alethefy", dir.path(), PublishAction::Deploy),
                PublishAction::Deploy,
            )
            .unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }

    #[test]
    fn deploy_build_commands_carry_the_long_timeout() {
        use crate::publish::{PUBLISH_DEPLOY_TIMEOUT, PUBLISH_SYNC_TIMEOUT};
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        let deploy = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let up = deploy
            .commands
            .iter()
            .find(|cmd| cmd.args.iter().any(|arg| arg == "up"))
            .expect("deploy up command");
        assert_eq!(up.timeout, Some(PUBLISH_DEPLOY_TIMEOUT));
        // The timeout is transport metadata only: it never renders
        // into the dry-run plan an operator reviews.
        assert!(!deploy.render().contains("1800"));
        // Sync-stage rsync carries the sync ceiling so multi-GB
        // trees under parallel-fleet contention do not fail closed
        // at the 60s transport default (`fleet-live-rollout`); the
        // mkdir probe keeps the default.
        let sync = adapter.plan(&req, PublishAction::Sync).unwrap();
        let rsync = sync
            .commands
            .iter()
            .find(|cmd| cmd.program == "rsync")
            .expect("sync rsync command");
        assert_eq!(rsync.timeout, Some(PUBLISH_SYNC_TIMEOUT));
        assert!(!sync.render().contains("600"));
    }

    #[test]
    fn dry_run_materialize_renders_a_preview_plan() {
        let dir = local_project();
        let adapter = adapter();
        let transport = RecordingTransport::new();
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, true).unwrap();
        assert_eq!(transport.command_count(), 0);
        let prepare = adapter.plan(&req, PublishAction::Prepare).unwrap();
        let rendered = prepare.render();
        assert!(rendered.contains("scp"));
        assert!(rendered.contains("/Users/allen/jenkins/runtime/port-registry.json.forge-new"));
        assert!(
            rendered.contains("/Users/allen/jenkins/runtime/alethefy/ports.compose.yml.forge-new")
        );
        assert!(rendered.contains("/bin/mv -f"));
        assert!(
            adapter
                .notes()
                .iter()
                .any(|note| note
                    .contains("dry-run: target registry and Compose config were not read"))
        );
    }

    #[test]
    fn apply_materialize_reads_the_target_and_ships_documents() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout:
                r#"{"projects":{"local:other":{"base":15000,"services":{"web:80/tcp":15000}}}}"#
                    .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: "other|127.0.0.1:15000->80/tcp\n".to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        let artifacts = adapter.staged_artifacts();
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/port-registry.json")));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/ports.compose.yml")));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/Caddyfile")));
        for artifact in &artifacts {
            assert!(
                artifact.local.is_file(),
                "{} missing",
                artifact.local.display()
            );
        }
        let deploy = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let rendered = deploy.render();
        assert!(rendered.contains("-p forge-alethefy"));
        assert!(rendered.contains("/Users/allen/jenkins/runtime/alethefy/ports.compose.yml"));
        assert!(rendered.contains("--project-directory /Users/allen/jenkins/projects/alethefy"));
        assert!(rendered.contains("up -d --build --remove-orphans"));
        assert!(rendered.contains("/usr/local/bin/docker compose -f /Users/allen/production/platform/compose.yml up -d --force-recreate"));
        assert!(rendered.contains("--env-file /Users/allen/production/secrets/alethefy/.env"));
        assert!(
            rendered.contains("--env-file /Users/allen/production/secrets/alethefy/.shared-db.env")
        );
        assert!(rendered.contains("-f /Users/allen/jenkins/runtime/alethefy/shared-db.compose.yml"));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
    }

    #[test]
    fn shared_db_declaration_adds_the_overlay_and_scale_args() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        let compose = r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#;
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: compose.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("alethefy", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(rendered.contains("-f /Users/allen/jenkins/runtime/alethefy/shared-db.compose.yml"));
    }

    #[test]
    fn standalone_database_compose_deploys_without_an_overlay() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        // Crossify-class compose: its own postgres, no application
        // service consuming the shared database.
        let compose = r#"{"services":{"postgres":{"image":"postgres:16-alpine","ports":[{"published":"5437","target":5432,"protocol":"tcp"}]},"minio":{"image":"minio/minio:RELEASE.2024-10-13T13-34-11Z"}}}"#;
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: compose.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        let req = request("crossify", dir.path(), PublishAction::All);
        adapter.materialize(&req, &transport, false).unwrap();
        assert!(!adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/shared-db.compose.yml")));
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("standalone database")));
        let rendered = adapter.plan(&req, PublishAction::Deploy).unwrap().render();
        assert!(!rendered.contains("shared-db.compose.yml"));
        assert!(!rendered.contains("--scale"));
    }

    #[test]
    fn missing_target_registry_bootstraps_a_new_registry() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "cat: no such file".to_string(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("bootstrapping a new registry")));
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/port-registry.json")));
    }

    #[test]
    fn unreadable_target_registry_is_a_typed_target_failure() {
        let dir = local_project();
        let adapter = adapter();
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "cat: permission denied".to_string(),
        }));
        let err = adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
        assert!(err.to_string().contains("port-registry.json"));
    }

    #[test]
    fn absent_target_compose_falls_back_to_the_local_checkout() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"projects":{}}"#.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "open /remote/dir/docker-compose.yml: no such file or directory".to_string(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("used the local checkout for port and overlay rendering")));
        assert!(adapter
            .staged_artifacts()
            .iter()
            .any(|artifact| artifact.remote.ends_with("/ports.compose.yml")));
    }

    #[test]
    fn captured_target_output_is_redacted_before_it_is_rendered() {
        let staging = tempfile::TempDir::new().unwrap();
        let dir = local_project();
        let adapter = adapter().with_staging_root(staging.path());
        let mut transport = RecordingTransport::new();
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"projects":{}}"#.to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 0,
            stdout: r#"{"services":{"alethefy":{"image":"alethefy:latest","environment":{"APP_PASSWORD":"password=letmein"},"ports":[{"published":"8000","target":8000,"protocol":"tcp"}]}}}"#
                .to_string(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        transport.push(Ok(CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }));
        adapter
            .materialize(
                &request("alethefy", dir.path(), PublishAction::All),
                &transport,
                false,
            )
            .unwrap();
        for note in adapter.notes() {
            assert!(!note.contains("letmein"));
        }
        assert!(adapter
            .notes()
            .iter()
            .any(|note| note.contains("allocating from the registry only")));
    }

    #[test]
    fn classification_reports_typed_recovery_hints() {
        let dir = local_project();
        let adapter = adapter();
        let req = request("alethefy", dir.path(), PublishAction::Sync);
        let plan = adapter.plan(&req, PublishAction::Sync).unwrap();
        let ok = CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert_eq!(adapter.classify(&plan, &ok).status, STATUS_DONE);
        let unreachable = CommandResult {
            status: 255,
            stdout: String::new(),
            stderr: "ssh: Could not resolve hostname mac".to_string(),
        };
        let classification = adapter.classify(&plan, &unreachable);
        assert_eq!(classification.status, "failed");
        assert!(classification
            .recovery
            .iter()
            .any(|hint| hint.contains("mac")));
    }

    #[test]
    fn compose_project_identity_is_the_forge_namespace() {
        assert_eq!(compose_project_identity("alethefy"), "forge-alethefy");
        assert_ne!(compose_project_identity("alethefy"), "jenkins-alethefy");
    }

    #[test]
    fn provider_container_identity_still_uses_the_revision_suffix() {
        assert_eq!(
            crate::publish::providers::compose_project_name(
                "alethefy",
                "0123456789abcdef0123456789abcdef01234567",
            ),
            "forge-alethefy-0123456789ab"
        );
    }
}
