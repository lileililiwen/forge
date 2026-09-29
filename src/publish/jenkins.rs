//! Jenkins/Mac adapter for the publish surface
//! (`jenkins-publish-integration`).
//!
//! All Jenkins-specific vocabulary lives here:
//!
//! - Default SSH target, remote paths, scripts directory
//! - Jenkins exit-code classification (3 = missing `.env`,
//!   4 = missing Jenkins credentials)
//! - Recovery hints for known failure modes
//! - Caddy subdomain format (`<project>.tooosall.uk`)
//!
//! Core ([`super`]) does not import anything from this module; it
//! sees only the [`PublishAdapter`] trait object. Swapping this
//! adapter for a GitHub-Actions or raw-SSH variant is a matter of
//! adding another file in this folder — the core stays untouched.

use std::env;
use std::ffi::OsString;

use crate::core::ForgeError;

use super::{
    Classification, CommandResult, CommandSpec, PublishAction, PublishAdapter, PublishRequest,
    StagePlan, PUBLISH_SYNC_TIMEOUT, STAGE_DB, STAGE_DEPLOY, STAGE_PREPARE, STAGE_SYNC,
};

// ---------------------------------------------------------------------------
// Adapter configuration
// ---------------------------------------------------------------------------

/// Environment variable selecting the SSH host alias.
const SSH_TARGET_ENV: &str = "FORGE_PUBLISH_SSH_TARGET";
/// Environment variable selecting the remote project root.
const REMOTE_ROOT_ENV: &str = "FORGE_PUBLISH_REMOTE_ROOT";
/// Environment variable selecting the remote scripts root.
const SCRIPTS_ROOT_ENV: &str = "FORGE_PUBLISH_SCRIPTS_ROOT";
/// Environment variable selecting the public domain.
const DOMAIN_ENV: &str = "FORGE_PUBLISH_DOMAIN";
/// Environment variable selecting the Caddy nav-host.
const NAV_HOST_ENV: &str = "FORGE_PUBLISH_NAV_HOST";

/// Default SSH host alias for the Mac.
const DEFAULT_SSH_TARGET: &str = "mac";
/// Default remote project directory on the Mac.
const DEFAULT_REMOTE_ROOT: &str = "/Users/allen/jenkins/projects";
/// Default remote scripts directory on the Mac.
const DEFAULT_SCRIPTS_ROOT: &str = "/Users/allen/jenkins/scripts";
/// Default public domain for subdomain routing.
const DEFAULT_DOMAIN: &str = "tooosall.uk";
/// Default nav-host for the Caddy index.
const DEFAULT_NAV_HOST: &str = "apps";

/// Configuration derived from the environment. Operators can
/// override every default through `FORGE_PUBLISH_*` variables so a
/// CI runner can target a different host without recompiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JenkinsConfig {
    pub ssh_target: String,
    pub remote_root: String,
    pub scripts_root: String,
    pub domain: String,
    pub nav_host: String,
}

impl JenkinsConfig {
    pub fn from_env() -> Self {
        Self::from_overrides(&Overrides::from_env())
    }

    pub fn from_overrides(o: &Overrides) -> Self {
        JenkinsConfig {
            ssh_target: o
                .ssh_target
                .clone()
                .unwrap_or_else(|| DEFAULT_SSH_TARGET.to_string()),
            remote_root: o
                .remote_root
                .clone()
                .unwrap_or_else(|| DEFAULT_REMOTE_ROOT.to_string()),
            scripts_root: o
                .scripts_root
                .clone()
                .unwrap_or_else(|| DEFAULT_SCRIPTS_ROOT.to_string()),
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
}

/// Optional environment overrides. Every field is `None` when the
/// corresponding env var is unset or empty. The struct is `pub` so
/// tests can construct fixtures without touching the process env.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Overrides {
    pub ssh_target: Option<String>,
    pub remote_root: Option<String>,
    pub scripts_root: Option<String>,
    pub domain: Option<String>,
    pub nav_host: Option<String>,
}

impl Overrides {
    pub fn from_env() -> Self {
        fn read(key: &str) -> Option<String> {
            env::var(key)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
        Overrides {
            ssh_target: read(SSH_TARGET_ENV),
            remote_root: read(REMOTE_ROOT_ENV),
            scripts_root: read(SCRIPTS_ROOT_ENV),
            domain: read(DOMAIN_ENV),
            nav_host: read(NAV_HOST_ENV),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter implementation
// ---------------------------------------------------------------------------

/// Jenkins adapter. Reads [`JenkinsConfig`] at construction; the
/// orchestrator never sees the config — only the trait methods.
pub struct JenkinsAdapter {
    config: JenkinsConfig,
}

impl JenkinsAdapter {
    pub fn new(config: JenkinsConfig) -> Self {
        JenkinsAdapter { config }
    }

    pub fn from_env() -> Self {
        JenkinsAdapter::new(JenkinsConfig::from_env())
    }

    // -- Path resolution: all path composition lives here --

    fn remote_project_dir(&self, project_id: &str) -> Result<RemotePath, ForgeError> {
        validate_project_id(project_id)?;
        Ok(RemotePath::new(format!(
            "{}/{}/",
            self.config.remote_root.trim_end_matches('/'),
            project_id
        )))
    }

    fn remote_script(&self, name: &str) -> Result<RemotePath, ForgeError> {
        validate_script_name(name)?;
        Ok(RemotePath::new(format!(
            "{}/{}",
            self.config.scripts_root.trim_end_matches('/'),
            name
        )))
    }
}

impl PublishAdapter for JenkinsAdapter {
    fn id(&self) -> &'static str {
        "jenkins"
    }
    fn label(&self) -> &'static str {
        "Jenkins/Mac"
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
            STAGE_SYNC => classify_sync(result),
            STAGE_DB => classify_db(result),
            STAGE_PREPARE => classify_prepare(result),
            STAGE_DEPLOY => classify_deploy(result),
            _ => fallback_classify(result),
        }
    }

    fn subdomain(&self, project_id: &str) -> Option<String> {
        Some(format!("{project_id}.{}", self.config.domain))
    }
}

// ---------------------------------------------------------------------------
// Stage plans
// ---------------------------------------------------------------------------

impl JenkinsAdapter {
    fn plan_sync(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        let remote = self.remote_project_dir(&request.project_id)?;
        let local = format!(
            "{}/",
            request
                .project_dir
                .display()
                .to_string()
                .trim_end_matches('/')
        );
        let ssh_target = self.config.ssh_target.as_str();
        let remote_target = format!("{}:{}/", ssh_target, remote.as_str().trim_end_matches('/'));

        // Command 1: ensure the remote directory exists.
        let mut mkdir = CommandSpec::new("ssh", "remote mkdir");
        mkdir.args.push(OsString::from(ssh_target));
        mkdir.args.push(OsString::from("mkdir"));
        mkdir.args.push(OsString::from("-p"));
        mkdir.args.push(OsString::from(remote.as_str()));

        // Command 2: rsync the source tree. Carries the sync
        // ceiling so multi-GB trees under parallel-fleet contention
        // do not fail closed at the interactive 60s transport
        // default (`fleet-live-rollout`).
        let mut rsync = CommandSpec::new("rsync", "project source sync");
        rsync.timeout = Some(PUBLISH_SYNC_TIMEOUT);
        rsync.args.push(OsString::from("-az"));
        rsync.args.push(OsString::from("--human-readable"));
        for excl in [".git/", "node_modules/", "target/", "dist/", "build/"] {
            rsync.args.push(OsString::from("--exclude"));
            rsync.args.push(OsString::from(excl));
        }
        rsync.args.push(OsString::from("-e"));
        rsync.args.push(OsString::from("ssh"));
        rsync.args.push(OsString::from(local));
        rsync.args.push(OsString::from(remote_target));

        Ok(StagePlan {
            stage: STAGE_SYNC.to_string(),
            commands: vec![mkdir, rsync],
            project_id: request.project_id.clone(),
        })
    }

    fn plan_prepare(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        let script = self.remote_script("project-ports.sh")?;
        let mut cmd = CommandSpec::new("ssh", "remote project-ports.sh");
        cmd.args
            .push(OsString::from(self.config.ssh_target.as_str()));
        cmd.args.push(OsString::from("bash"));
        cmd.args.push(OsString::from(script.as_str()));
        cmd.args.push(OsString::from(request.project_id.as_str()));

        Ok(StagePlan {
            stage: STAGE_PREPARE.to_string(),
            commands: vec![cmd],
            project_id: request.project_id.clone(),
        })
    }

    fn plan_db(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        // Provisioning shared PostgreSQL runs the existing
        // shared-postgres.sh twice: once to ensure the container is
        // running, once to provision the project-specific database
        // and write `.shared-db.env`. The deploy stage consumes
        // that env file, so this stage must precede `deploy`.
        let script = self.remote_script("shared-postgres.sh")?;
        let target = self.config.ssh_target.as_str();
        let mut start = CommandSpec::new("ssh", "remote shared-postgres.sh start");
        start.args.push(OsString::from(target));
        start.args.push(OsString::from("bash"));
        start.args.push(OsString::from(script.as_str()));
        start.args.push(OsString::from("start"));
        start.args.push(OsString::from("production"));

        let mut provision = CommandSpec::new("ssh", "remote shared-postgres.sh provision");
        provision.args.push(OsString::from(target));
        provision.args.push(OsString::from("bash"));
        provision.args.push(OsString::from(script.as_str()));
        provision.args.push(OsString::from("provision"));
        provision.args.push(OsString::from("production"));
        provision
            .args
            .push(OsString::from(request.project_id.as_str()));

        Ok(StagePlan {
            stage: STAGE_DB.to_string(),
            commands: vec![start, provision],
            project_id: request.project_id.clone(),
        })
    }

    fn plan_deploy(&self, request: &PublishRequest) -> Result<StagePlan, ForgeError> {
        // The Mac's deploy pipeline lives in `project-action.sh`,
        // not `project.sh`. `project-action.sh` accepts the project
        // name and the action verb and runs docker compose under
        // the hood.
        let script = self.remote_script("project-action.sh")?;
        let mut cmd = CommandSpec::new("ssh", "remote project-action.sh deploy");
        cmd.args
            .push(OsString::from(self.config.ssh_target.as_str()));
        cmd.args.push(OsString::from("bash"));
        cmd.args.push(OsString::from(script.as_str()));
        cmd.args.push(OsString::from(request.project_id.as_str()));
        cmd.args.push(OsString::from("deploy"));

        Ok(StagePlan {
            stage: STAGE_DEPLOY.to_string(),
            commands: vec![cmd],
            project_id: request.project_id.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Classification (Jenkins exit-code vocabulary)
// ---------------------------------------------------------------------------

fn classify_sync(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("project source tree synced to Mac");
    }
    let recovery = match result.status {
        // ssh exit 12: "ssh: Could not resolve hostname"
        255 | 12 => vec![
            format!(
                "verify SSH alias `{}` resolves in ~/.ssh/config",
                env::var(SSH_TARGET_ENV).unwrap_or_else(|_| DEFAULT_SSH_TARGET.to_string())
            ),
            "partial state may exist on the Mac; rerun `forge publish sync` to repair".to_string(),
        ],
        _ => vec!["inspect the ssh/rsync output above for the underlying cause".to_string()],
    };
    Classification::failed("sync failed", recovery)
}

fn classify_prepare(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("ports allocated, .env ensured");
    }
    Classification::failed(
        "prepare failed",
        vec!["rerun `forge publish prepare` after configuring the project on the Mac".to_string()],
    )
}

fn classify_db(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("shared PostgreSQL database provisioned");
    }
    let recovery = match result.status {
        // shared-postgres.sh exit 1: container not running. The start
        // subcommand should have started it; investigate Docker.
        1 => vec![
            "verify Docker is running on the Mac: `docker ps` should list production-postgres"
                .to_string(),
            "rerun `forge publish db <project>`".to_string(),
        ],
        _ => vec![
            "check `shared-postgres.sh status` on the Mac".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("shared PostgreSQL provisioning failed", recovery)
}

fn classify_deploy(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("Jenkins deploy job triggered");
    }
    let recovery = match result.status {
        // project-action.sh exit 2: invalid arguments (project name
        // or action verb). Means our CLI passed a bad value.
        2 => vec![
            "verify the project id is kebab/snake-case (letters, digits, dash, underscore)"
                .to_string(),
        ],
        // project-action.sh exit 1: missing project dir, compose file,
        // or port override. Means the prepare stage did not run
        // successfully first.
        1 => vec![
            "run `forge publish prepare <project>` first to allocate ports".to_string(),
            "verify the project directory and docker-compose.yml exist on the Mac".to_string(),
        ],
        _ => vec![
            "check the project-action.sh log on the Mac".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("deploy rejected the request", recovery)
}

fn fallback_classify(result: &CommandResult) -> Classification {
    if result.success() {
        Classification::done("stage completed")
    } else {
        Classification::failed(
            "stage failed",
            vec![format!("inspect exit {}", result.status)],
        )
    }
}

// ---------------------------------------------------------------------------
// Validators and helpers
// ---------------------------------------------------------------------------

/// A validated remote path. The wrapper type prevents accidental
/// concatenation outside the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePath(String);

impl RemotePath {
    pub fn new(value: impl Into<String>) -> Self {
        RemotePath(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RemotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
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

fn validate_script_name(name: &str) -> Result<(), ForgeError> {
    if name.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "script name must not be empty".to_string(),
        });
    }
    if name.contains('/') || name.contains("..") {
        return Err(ForgeError::PublishInvalid {
            reason: format!("script name `{name}` must not contain `/` or `..`"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::STATUS_DONE;
    use std::path::Path;

    fn fixture() -> JenkinsAdapter {
        JenkinsAdapter::new(JenkinsConfig::from_overrides(&Overrides::default()))
    }

    fn req_with_id(id: &str) -> PublishRequest {
        PublishRequest {
            project_id: id.to_string(),
            project_dir: Path::new("/tmp").to_path_buf(),
            action: PublishAction::Sync,
            dry_run: true,
        }
    }

    #[test]
    fn config_defaults_match_module_constants() {
        let cfg = JenkinsConfig::from_overrides(&Overrides::default());
        assert_eq!(cfg.ssh_target, DEFAULT_SSH_TARGET);
        assert_eq!(cfg.remote_root, DEFAULT_REMOTE_ROOT);
        assert_eq!(cfg.scripts_root, DEFAULT_SCRIPTS_ROOT);
        assert_eq!(cfg.domain, DEFAULT_DOMAIN);
        assert_eq!(cfg.nav_host, DEFAULT_NAV_HOST);
    }

    #[test]
    fn config_overrides_apply_one_field_at_a_time() {
        let o = Overrides {
            ssh_target: Some("prod-jenkins".to_string()),
            ..Overrides::default()
        };
        let cfg = JenkinsConfig::from_overrides(&o);
        assert_eq!(cfg.ssh_target, "prod-jenkins");
        assert_eq!(cfg.remote_root, DEFAULT_REMOTE_ROOT);
    }

    #[test]
    fn remote_project_dir_preserves_trailing_separator() {
        let adapter = fixture();
        let remote = adapter.remote_project_dir("demo").unwrap();
        assert!(remote.as_str().ends_with('/'));
        assert!(remote.as_str().contains("/demo/"));
    }

    #[test]
    fn remote_project_dir_rejects_invalid_id() {
        let adapter = fixture();
        assert!(adapter.remote_project_dir("").is_err());
        assert!(adapter.remote_project_dir("demo.app").is_err());
        assert!(adapter.remote_project_dir("demo with space").is_err());
    }

    #[test]
    fn remote_script_rejects_path_traversal() {
        let adapter = fixture();
        assert!(adapter.remote_script("").is_err());
        assert!(adapter.remote_script("../escape").is_err());
        assert!(adapter.remote_script("sub/dir").is_err());
        assert!(adapter.remote_script("project-ports.sh").is_ok());
    }

    #[test]
    fn adapter_id_and_label_are_stable() {
        let adapter = fixture();
        assert_eq!(adapter.id(), "jenkins");
        assert_eq!(adapter.label(), "Jenkins/Mac");
    }

    #[test]
    fn plan_sync_renders_expected_command_shape() {
        use super::PUBLISH_SYNC_TIMEOUT;
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Sync).unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("ssh mac mkdir"));
        assert!(rendered.contains("rsync"));
        assert!(rendered.contains("/Users/allen/jenkins/projects/demo/"));
        assert!(rendered.contains(".git/"));
        // Sync-stage rsync carries the sync ceiling
        // (`fleet-live-rollout`); the mkdir probe keeps the
        // transport default and the timeout never renders.
        let rsync = plan
            .commands
            .iter()
            .find(|cmd| cmd.program == "rsync")
            .expect("sync rsync command");
        assert_eq!(rsync.timeout, Some(PUBLISH_SYNC_TIMEOUT));
        assert!(!rendered.contains("600"));
    }

    #[test]
    fn plan_prepare_renders_expected_command_shape() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Prepare).unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("project-ports.sh"));
        assert!(rendered.contains("demo"));
    }

    #[test]
    fn plan_db_renders_expected_command_shape() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Db).unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("shared-postgres.sh"));
        assert!(rendered.contains("start production"));
        assert!(rendered.contains("provision production demo"));
    }

    #[test]
    fn classify_db_success_yields_done() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Db).unwrap();
        let ok = CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        let cls = adapter.classify(&plan, &ok);
        assert_eq!(cls.status, STATUS_DONE);
    }

    #[test]
    fn classify_db_failure_suggests_docker_check() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Db).unwrap();
        let bad = CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "container not running".to_string(),
        };
        let cls = adapter.classify(&plan, &bad);
        assert_eq!(cls.status, "failed");
        assert!(cls.recovery.iter().any(|h| h.contains("Docker")));
    }

    #[test]
    fn plan_deploy_renders_expected_command_shape() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let rendered = plan.render();
        assert!(rendered.contains("project-action.sh"));
        assert!(rendered.contains("demo"));
        assert!(rendered.contains("deploy"));
    }

    #[test]
    fn plan_all_is_rejected() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let err = adapter.plan(&req, PublishAction::All).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn classify_sync_success_yields_done() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Sync).unwrap();
        let ok = CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        };
        let cls = adapter.classify(&plan, &ok);
        assert_eq!(cls.status, STATUS_DONE);
        assert!(cls.recovery.is_empty());
    }

    #[test]
    fn classify_sync_failure_reports_recovery_hint() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Sync).unwrap();
        let bad = CommandResult {
            status: 255,
            stdout: String::new(),
            stderr: "ssh: Could not resolve hostname mac".to_string(),
        };
        let cls = adapter.classify(&plan, &bad);
        assert_eq!(cls.status, "failed");
        assert!(!cls.recovery.is_empty());
    }

    #[test]
    fn classify_deploy_exit_2_suggests_valid_project_id() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let bad = CommandResult {
            status: 2,
            stdout: String::new(),
            stderr: "Invalid project name".to_string(),
        };
        let cls = adapter.classify(&plan, &bad);
        assert_eq!(cls.status, "failed");
        assert!(cls.recovery.iter().any(|h| h.contains("kebab")));
    }

    #[test]
    fn classify_deploy_exit_1_suggests_prepare_first() {
        let adapter = fixture();
        let req = req_with_id("demo");
        let plan = adapter.plan(&req, PublishAction::Deploy).unwrap();
        let bad = CommandResult {
            status: 1,
            stdout: String::new(),
            stderr: "Project not found".to_string(),
        };
        let cls = adapter.classify(&plan, &bad);
        assert_eq!(cls.status, "failed");
        assert!(cls.recovery.iter().any(|h| h.contains("prepare")));
    }

    #[test]
    fn subdomain_format_matches_domain_config() {
        let adapter = JenkinsAdapter::new(JenkinsConfig {
            domain: "test.example".to_string(),
            ..JenkinsConfig::from_overrides(&Overrides::default())
        });
        assert_eq!(
            adapter.subdomain("demo"),
            Some("demo.test.example".to_string())
        );
    }

    #[test]
    fn overrides_from_env_uses_default_when_unset() {
        for key in [
            SSH_TARGET_ENV,
            REMOTE_ROOT_ENV,
            SCRIPTS_ROOT_ENV,
            DOMAIN_ENV,
            NAV_HOST_ENV,
        ] {
            env::remove_var(key);
        }
        let o = Overrides::from_env();
        assert!(o.ssh_target.is_none());
        assert!(o.domain.is_none());
    }
}
