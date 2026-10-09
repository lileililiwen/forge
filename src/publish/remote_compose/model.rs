//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    CommandSpec, PublishRequest, SshTransport, StagePlan, PUBLISH_DEPLOY_TIMEOUT,
    PUBLISH_SYNC_TIMEOUT, STAGE_DB, STAGE_DEPLOY, STAGE_PREPARE, STAGE_SYNC,
};
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::publish::caddy;
use crate::publish::db_overlay;
use crate::publish::port_allocator::{
    self, PortAllocator, PortRange, PortRegistry, DEFAULT_BIND_ADDRESS,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

use super::constants::{
    COMPOSE_CANDIDATES, DEFAULT_DOMAIN, DEFAULT_NAV_HOST, DEFAULT_PLATFORM_ROOT,
    DEFAULT_REMOTE_ROOT, DEFAULT_RUNTIME_ROOT, DEFAULT_SECRETS_ROOT, DEFAULT_SHARED_INFRA_ROOT,
    DEFAULT_SSH_TARGET, DOMAIN_ENV, MAX_COMPOSE_BYTES, MAX_DOCKER_PS_BYTES, NAV_HOST_ENV,
    PLATFORM_ROOT_ENV, REMOTE_MKDIR, REMOTE_MV, REMOTE_ROOT_ENV, RUNTIME_ROOT_ENV,
    SECRETS_ROOT_ENV, SHARED_INFRA_ROOT_ENV, SSH_TARGET_ENV, STAGED_SUFFIX,
};
use super::stages::{
    compose_project_identity, docker_command, file_name, profile_args, read_local_compose,
    validate_project_id,
};

/// The decoupled publish adapter. State is interior-mutable because
/// the orchestrator holds the adapter behind `&dyn PublishAdapter` and
/// calls [`PublishAdapter::materialize`] before planning.
pub struct RemoteComposeAdapter {
    pub(super) config: RemoteConfig,
    pub(super) staging_root: PathBuf,
    pub(super) state: RefCell<RemoteState>,
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
    pub(super) fn remote_project_dir(&self, project_id: &str) -> String {
        self.config.remote_project_dir(project_id)
    }
    pub(super) fn compose_file(&self) -> Result<String, ForgeError> {
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
    pub(super) fn ship(&self, local: &Path, remote: &str) -> Vec<CommandSpec> {
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
impl RemoteComposeAdapter {
    pub(super) fn plan_sync(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
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
        let mut exclusions: Vec<String> = Vec::new();
        for exclusion in [
            ".git/",
            "node_modules/",
            "target/",
            "dist/",
            "build/",
            "obj/",
            "appendonlydir/",
            "*.rdb",
        ] {
            exclusions.push("--exclude".to_string());
            exclusions.push(exclusion.to_string());
        }
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
    pub(super) fn plan_prepare(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
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
    pub(super) fn plan_db(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
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
    pub(super) fn plan_deploy(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
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
                .with_timeout(PUBLISH_DEPLOY_TIMEOUT)
                .with_exclusive(),
        );
        Ok(StagePlan {
            stage: STAGE_DEPLOY.to_string(),
            commands,
            project_id: request.project_id.clone(),
        })
    }
    /// Target-local env files referenced by the deploy, in Compose's
    /// own precedence order. Their contents never reach Linux.
    pub(super) fn deploy_env_files(&self, project_id: &str) -> Vec<String> {
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
impl RemoteComposeAdapter {
    pub(super) fn read_registry(
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
    pub(super) fn resolve_compose_file(
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
    pub(super) fn read_compose_config(
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
        let stderr = redact_credentials(&result.stderr);
        if stderr.to_lowercase().contains("no such file") && !compose_file.is_empty() {
            if let Some(document) = read_local_compose(project_dir, &compose_file)? {
                state
                    .notes
                    .push(
                        format!(
                            "target Compose file {path} is absent; used the local checkout for port and overlay rendering"
                        ),
                    );
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
    pub(super) fn read_unavailable_ports(
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
    pub(super) fn probe_target_file(&self, transport: &dyn SshTransport, path: &str) -> bool {
        let command = CommandSpec::new("ssh", "probe target-local file")
            .arg(self.config.ssh_target.as_str())
            .arg("test")
            .arg("-f")
            .arg(path);
        matches!(transport.run(command), Ok(result) if result.success())
    }
    pub(super) fn render_artifacts(
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
            state
                .notes
                .push(
                    format!(
                        "project `{}` runs standalone database service(s) {}; deployed without a shared-database overlay",
                        project_id, databases.join(", ")
                    ),
                );
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
            state
                .notes
                .push(
                    "dry-run: documents are a local preview only; no subprocess was spawned and the target is unchanged"
                        .to_string(),
                );
        }
        Ok(())
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
pub(super) struct RemoteState {
    pub(super) registry: PortRegistry,
    pub(super) compose_file: Option<String>,
    pub(super) compose: Value,
    pub(super) unavailable: BTreeSet<u32>,
    pub(super) shared_db_env: bool,
    pub(super) project_env: bool,
    /// True when a shared-database overlay was actually rendered and
    /// shipped. False when the project runs standalone databases
    /// (deployed as-is) or declares no shared-DB intent.
    pub(super) shared_db_overlay: bool,
    /// Compose profiles declared by the project's own Compose file.
    /// A bare `up` selects no profile-gated service (`no service
    /// selected`), so deploys — and the `config` read that feeds
    /// port allocation — enable every declared profile: Forge
    /// publishes the whole project, and the legacy lane never
    /// selected profiles either.
    pub(super) profiles: Vec<String>,
    pub(super) scale_args: Vec<String>,
    pub(super) artifacts: Vec<StagedArtifact>,
    pub(super) notes: Vec<String>,
}
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
