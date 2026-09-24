//! Adapter-based deployment and observed runtime state (`adapter-deployment`).
//!
//! Core owns the typed deployment contract. v0.1.0 supports:
//!
//! - [`DeployConfig`] parses the manifest's `deployment` block,
//!   refuses empty or duplicate targets, accepts only the
//!   supported adapter kinds (`local`, `docker-compose`;
//!   `ssh` is planned) and confines artifact paths to the
//!   project directory.
//! - [`prepare_deploy`] captures the named target, the
//!   artifact identity (path, content hash), the working-tree
//!   revision and the configured health check into a
//!   reviewable [`DeployPlan`]. No side effect runs from
//!   `prepare`.
//! - [`apply_deploy`] refuses without `--confirm`; walks the
//!   per-target adapter invocation through the configured
//!   `FORGE_DEPLOYER_BIN` binary, captures the timestamped
//!   health observation and persists a [`DeployState`]
//!   under `.forge/deploy/<project-id>/<deploy-id>/state.json`.
//! - [`observe_deploy`] re-runs the health check after a
//!   successful apply (or on demand) and updates the same
//!   state: a passing observation records `running`; a
//!   failing observation records `failed`; an unreachable
//!   target leaves the last successful observation as
//!   `last_observed_running` and the current state as
//!   `unknown` (R2 boundary: disconnected is unknown, not
//!   offline proof).
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §30, §34, §43 require
//! simple deployment targets and observed runtime state. The
//! contract is independent of any real Docker daemon, SSH
//! endpoint or external coordinator: a missing adapter
//! binary, an unsupported target, a stale plan or a target
//! that refuses the connection surfaces as a typed
//! `error[...]` response before any health observation is
//! recorded.
//!
//! ## Persistence
//!
//! [`DeployState`] lives under
//! `.forge/deploy/<project-id>/<deploy-id>/state.json` so a
//! retry sees exactly which target already delivered and
//! which needs another attempt. The state is local
//! evidence, not a record of authority: a successful run
//! overwrites the prior entry, a failed run leaves the last
//! good observation untouched. The Core registry's
//! `operations` table receives one `deploy` row per
//! prepare/apply/observe with a `done`/`partial`/`blocked`
//! summary.
//!
//! ## Risk model
//!
//! A remote write is irreversible. The contract refuses to
//! apply a deploy without an explicit `--confirm`; the
//! contract refuses to overwrite a successful health
//! observation with a stale one. Credentials embedded in
//! evidence are redacted through
//! [`crate::policy::redact_credentials`].
//!
//! Real provider integration is out of scope for v0.1.0:
//! the `local` and `docker-compose` adapters reuse the same
//! `FORGE_DEPLOYER_BIN` environment-variable pattern the
//! docs and policy contracts use, so a fixture binary
//! stands in for a real `docker compose` or `scp` round
//! trip.

pub mod engine;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::manifest::{DeploymentHealthMeta, DeploymentMeta, Manifest};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

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

/// Stable target kinds. Only `local` and `docker-compose`
/// are supported in v0.1.0; `ssh` is planned (refused with
/// a typed `unavailable` boundary so a manifest cannot
/// silently introduce an unsupportable target).
pub const TARGET_LOCAL: &str = "local";
pub const TARGET_DOCKER_COMPOSE: &str = "docker-compose";
pub const TARGET_SSH: &str = "ssh";

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

/// Hex SHA-256 over bytes. Reused for artifact identity.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// One typed target from the manifest's `deployment.targets`
/// list after validation. Untyped entries are rejected
/// before any adapter invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployTargetSpec {
    pub name: String,
    pub kind: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub path: Option<String>,
    pub service: Option<String>,
    pub note: Option<String>,
}

/// Validated deployment config parsed from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Adapter kind declared on the manifest's `deployment.type`
    /// (legacy single-target form) or on the chosen target.
    /// `mixed` when the manifest lists targets of different
    /// adapter kinds.
    pub adapter: String,
    /// Default target name. Always populated: a single-target
    /// manifest's only entry is its own default.
    pub default_target: String,
    /// Resolved typed targets.
    pub targets: Vec<DeployTargetSpec>,
    /// Artifact path (relative to the project root) consumed
    /// by the adapter. None when the adapter kind does not
    /// need an artifact.
    pub artifact: Option<String>,
    /// Per-target health check configuration. None disables
    /// the observation step and the deploy is reported as
    /// `pending` until a manual `forge deploy observe` is
    /// requested.
    pub health: Option<DeployHealthSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployHealthSpec {
    pub kind: String,
    pub service: Option<String>,
    pub url: Option<String>,
    pub process: Option<String>,
    pub interval_seconds: Option<u32>,
}

impl DeployConfig {
    /// Parse the raw [`DeploymentMeta`] into a typed
    /// configuration. Defaults are applied here so the rest
    /// of the contract can read every field without checking
    /// for absence.
    pub fn from_manifest_meta(meta: &DeploymentMeta) -> Result<Self, ForgeError> {
        let mut typed_targets: Vec<DeployTargetSpec> = Vec::new();
        for (idx, entry) in meta.targets.iter().enumerate() {
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| format!("target-{idx}"));
            let kind = entry.kind.clone().ok_or_else(|| ForgeError::DeployInvalid {
                reason: format!(
                    "deployment.targets[{idx}] (`{name}`) is missing a `kind`; expected one of `{TARGET_LOCAL}`, `{TARGET_DOCKER_COMPOSE}`, `{TARGET_SSH}`"
                ),
            })?;
            validate_target_kind(&kind)?;
            validate_target_name(&name)?;
            if typed_targets
                .iter()
                .any(|t: &DeployTargetSpec| t.name == name)
            {
                return Err(ForgeError::DeployInvalid {
                    reason: format!("deployment.targets entry `{name}` is duplicated"),
                });
            }
            typed_targets.push(DeployTargetSpec {
                name,
                kind,
                host: entry.host.clone(),
                user: entry.user.clone(),
                path: entry.path.clone(),
                service: entry.service.clone(),
                note: entry.note.clone(),
            });
        }
        // Legacy single-target form: synthesise one target
        // from `deployment.type` + `deployment.target`. The
        // contract refuses to mix the legacy form with the
        // typed targets list so a manifest cannot double
        // declare.
        if typed_targets.is_empty() {
            if let Some(deploy_type) = &meta.deploy_type {
                validate_target_kind(deploy_type)?;
                let name = meta.target.clone().unwrap_or_else(|| "default".to_string());
                validate_target_name(&name)?;
                typed_targets.push(DeployTargetSpec {
                    name: name.clone(),
                    kind: deploy_type.clone(),
                    host: None,
                    user: None,
                    path: None,
                    service: None,
                    note: Some(
                        "derived from legacy `deployment.type` + `deployment.target`".to_string(),
                    ),
                });
            }
        } else if meta.deploy_type.is_some() || meta.target.is_some() {
            return Err(ForgeError::DeployInvalid {
                reason: "deployment block mixes typed `targets[]` with legacy `type`/`target`; declare one or the other".to_string(),
            });
        }
        if typed_targets.is_empty() {
            return Err(ForgeError::DeployInvalid {
                reason: "deployment block declares no targets; declare at least one `deployment.targets[]` entry or a legacy `type`".to_string(),
            });
        }
        // Default target resolution: the manifest's
        // `deployment.default` wins, otherwise a single
        // target is its own default, otherwise the first
        // typed target is used.
        let default_target = if let Some(default) = &meta.default {
            if !typed_targets.iter().any(|t| &t.name == default) {
                return Err(ForgeError::DeployInvalid {
                    reason: format!(
                        "deployment.default `{default}` does not match any declared target"
                    ),
                });
            }
            default.clone()
        } else if typed_targets.len() == 1 {
            typed_targets[0].name.clone()
        } else {
            return Err(ForgeError::DeployInvalid {
                reason: format!(
                    "deployment block declares {} targets but no `default`; pass `--target` or set `deployment.default`",
                    typed_targets.len()
                ),
            });
        };
        let adapter = if typed_targets
            .iter()
            .all(|t| t.kind == typed_targets[0].kind)
        {
            typed_targets[0].kind.clone()
        } else {
            "mixed".to_string()
        };
        let artifact = match &meta.artifact {
            Some(path) => {
                if !lexically_inside(".", path) {
                    return Err(ForgeError::DeployInvalid {
                        reason: format!(
                            "deployment.artifact `{path}` resolves outside the project"
                        ),
                    });
                }
                if path.trim().is_empty() {
                    return Err(ForgeError::DeployInvalid {
                        reason: "deployment.artifact must not be empty".to_string(),
                    });
                }
                Some(path.clone())
            }
            None => None,
        };
        let health = meta
            .health
            .as_ref()
            .map(DeployHealthSpec::from_manifest_meta)
            .transpose()?;
        Ok(DeployConfig {
            adapter,
            default_target,
            targets: typed_targets,
            artifact,
            health,
        })
    }

    /// Look up a target by name. The boundary scenario
    /// (`--target <unknown>`) refuses with a typed error
    /// rather than choosing an arbitrary target.
    pub fn target(&self, name: &str) -> Result<&DeployTargetSpec, ForgeError> {
        self.targets
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| ForgeError::DeployInvalid {
                reason: format!(
                    "target `{name}` is not declared in `deployment.targets[]`; declared: {}",
                    self.targets
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
    }
}

impl DeployHealthSpec {
    fn from_manifest_meta(meta: &DeploymentHealthMeta) -> Result<Self, ForgeError> {
        let kind = meta.kind.clone().ok_or_else(|| ForgeError::DeployInvalid {
            reason: "deployment.health.kind is required when `health` is declared".to_string(),
        })?;
        match kind.as_str() {
            HEALTH_DOCKER => {
                if meta.service.as_deref().unwrap_or("").is_empty() {
                    return Err(ForgeError::DeployInvalid {
                        reason: format!(
                            "deployment.health.service is required for `{HEALTH_DOCKER}` health checks"
                        ),
                    });
                }
            }
            HEALTH_HTTP => {
                if meta.url.as_deref().unwrap_or("").is_empty() {
                    return Err(ForgeError::DeployInvalid {
                        reason: format!(
                            "deployment.health.url is required for `{HEALTH_HTTP}` health checks"
                        ),
                    });
                }
            }
            HEALTH_PROCESS => {
                if meta.process.as_deref().unwrap_or("").is_empty() {
                    return Err(ForgeError::DeployInvalid {
                        reason: format!(
                            "deployment.health.process is required for `{HEALTH_PROCESS}` health checks"
                        ),
                    });
                }
            }
            other => {
                return Err(ForgeError::DeployInvalid {
                    reason: format!(
                        "deployment.health.kind `{other}` is not supported; expected one of `{HEALTH_DOCKER}`, `{HEALTH_HTTP}`, `{HEALTH_PROCESS}`"
                    ),
                });
            }
        }
        Ok(DeployHealthSpec {
            kind,
            service: meta.service.clone(),
            url: meta.url.clone(),
            process: meta.process.clone(),
            interval_seconds: meta.interval_seconds,
        })
    }
}

fn validate_target_kind(kind: &str) -> Result<(), ForgeError> {
    match kind {
        TARGET_LOCAL | TARGET_DOCKER_COMPOSE | TARGET_SSH => Ok(()),
        other => Err(ForgeError::DeployInvalid {
            reason: format!(
                "deployment target kind `{other}` is not supported; expected one of `{TARGET_LOCAL}`, `{TARGET_DOCKER_COMPOSE}`, `{TARGET_SSH}`"
            ),
        }),
    }
}

fn validate_target_name(name: &str) -> Result<(), ForgeError> {
    if name.is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "deployment target name must not be empty".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "deployment target name `{name}` must be kebab-case (lowercase letters, digits, single dashes)"
            ),
        });
    }
    Ok(())
}

/// Build a [`DeployConfig`] from a manifest's deployment
/// section. Convenience for callers that already have the
/// manifest.
pub fn deploy_config_from_manifest(manifest: &Manifest) -> Result<DeployConfig, ForgeError> {
    match manifest.deployment.as_ref() {
        Some(meta) => DeployConfig::from_manifest_meta(meta),
        None => Err(ForgeError::DeployInvalid {
            reason: format!(
                "project `{}` has no `deployment` section; declare one with at least one `deployment.targets[]` entry",
                manifest.project.id
            ),
        }),
    }
}

/// Deploy identity. The id is derived from the project id,
/// the target name and the first 12 hex characters of the
/// source revision hash. Two deploy attempts on the same
/// project at the same target and source revision resolve to
/// the same id; a different revision, target or semver
/// produces a different id, so a prepared plan is never
/// silently reused for a different artifact.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployIdentity {
    pub project_id: String,
    pub target: String,
    pub source_revision: String,
    pub id: String,
}

impl DeployIdentity {
    pub fn derive(project_id: &str, target: &str, source_revision: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(project_id.as_bytes());
        hasher.update(b"@");
        hasher.update(target.as_bytes());
        hasher.update(b"@");
        hasher.update(source_revision.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        let short = &digest[..12.min(digest.len())];
        let id = format!("{project_id}-{target}-{short}");
        DeployIdentity {
            project_id: project_id.to_string(),
            target: target.to_string(),
            source_revision: source_revision.to_string(),
            id,
        }
    }
}

/// Captured artifact identity. The full content is captured
/// at prepare time so a re-run never reads a different file
/// after the working tree has moved on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedArtifact {
    pub path: String,
    pub content_hash: String,
    pub byte_size: u64,
}

/// Deploy request from the transport layer. The CLI/MCP
/// build this; Core owns validation and execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployRequest {
    pub project_id: String,
    pub target: String,
    pub confirm: bool,
    pub dry_run: bool,
}

impl DeployRequest {
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.project_id.trim().is_empty() {
            return Err(ForgeError::DeployInvalid {
                reason: "project id must not be empty".to_string(),
            });
        }
        if self.target.trim().is_empty() {
            return Err(ForgeError::DeployInvalid {
                reason: "deploy target must not be empty; pass `--target <name>` or set `deployment.default`".to_string(),
            });
        }
        Ok(())
    }
}

/// Read-only deploy plan. The transport renders this
/// before any side effect runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeployPlan {
    pub contract: String,
    pub project_id: String,
    pub identity: DeployIdentity,
    pub target: DeployTargetSpec,
    pub artifact: Option<CapturedArtifact>,
    pub health: Option<DeployHealthSpec>,
    pub note: String,
    pub ready: bool,
}

impl DeployPlan {
    pub fn healthy(&self) -> bool {
        self.ready
    }
}

/// Per-stage outcome of an apply run. Statuses are stable
/// across CLI, MCP and journal rows; the contract owns the
/// labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployStageOutcome {
    pub stage: String,
    pub target: String,
    pub status: String,
    pub note: String,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthObservation {
    pub status: String,
    pub detail: String,
    pub observed_at: String,
    pub evidence: Vec<String>,
}

impl HealthObservation {
    pub fn unknown(detail: impl Into<String>, evidence: Vec<String>, observed_at: String) -> Self {
        HealthObservation {
            status: STATUS_UNKNOWN.to_string(),
            detail: detail.into(),
            evidence,
            observed_at,
        }
    }

    pub fn running(detail: impl Into<String>, evidence: Vec<String>, observed_at: String) -> Self {
        HealthObservation {
            status: STATUS_RUNNING.to_string(),
            detail: detail.into(),
            evidence,
            observed_at,
        }
    }

    pub fn failed(detail: impl Into<String>, evidence: Vec<String>, observed_at: String) -> Self {
        HealthObservation {
            status: STATUS_FAILED.to_string(),
            detail: detail.into(),
            evidence,
            observed_at,
        }
    }
}

/// Aggregate deploy report from a prepare/apply/observe run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeployReport {
    pub contract: String,
    pub project_id: String,
    pub identity: DeployIdentity,
    pub target: DeployTargetSpec,
    pub adapter: String,
    pub artifact: Option<CapturedArtifact>,
    pub health: Option<DeployHealthSpec>,
    pub stages: Vec<DeployStageOutcome>,
    pub observation: Option<HealthObservation>,
    pub dry_run: bool,
    pub state_path: String,
    pub note: String,
    pub healthy: bool,
}

impl DeployReport {
    pub fn healthy(&self) -> bool {
        self.healthy
    }
}

/// Persisted deploy state. Tracks which target already
/// delivered and the last successful health observation.
/// Stored under
/// `.forge/deploy/<project-id>/<deploy-id>/state.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployState {
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub identity: DeployIdentity,
    #[serde(default)]
    pub target: DeployTargetSpec,
    #[serde(default)]
    pub adapter: String,
    #[serde(default)]
    pub artifact: Option<CapturedArtifact>,
    #[serde(default)]
    pub health: Option<DeployHealthSpec>,
    #[serde(default)]
    pub stage_outcomes: Vec<DeployStageOutcome>,
    #[serde(default)]
    pub last_observation: Option<HealthObservation>,
    /// The last `running` observation this deploy recorded.
    /// An unreachable or unrecognized target never erases
    /// it: `current_state` still reports `unknown` for the
    /// live observation while prior proof of health remains
    /// attributable (R2 boundary: disconnected is unknown,
    /// not offline proof).
    #[serde(default)]
    pub last_observed_running: Option<HealthObservation>,
    #[serde(default)]
    pub last_run_at: String,
}

impl DeployState {
    /// Current observed state: `running` when the last
    /// observation was `running` and not stale; `failed`
    /// when the last observation was `failed`; `unknown`
    /// when the target is unreachable (R2 boundary); a
    /// placeholder when no observation has been recorded
    /// yet.
    pub fn current_state(&self) -> String {
        match &self.last_observation {
            Some(obs) if obs.status == STATUS_RUNNING => STATUS_RUNNING.to_string(),
            Some(obs) if obs.status == STATUS_FAILED => STATUS_FAILED.to_string(),
            Some(obs) if obs.status == STATUS_UNKNOWN => STATUS_UNKNOWN.to_string(),
            Some(_) => STATUS_UNKNOWN.to_string(),
            None => STATUS_PENDING.to_string(),
        }
    }
}

/// Lightweight deploy list entry for `forge deploy list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployListEntry {
    pub deploy_id: String,
    pub project_id: String,
    pub target: String,
    pub source_revision: String,
    pub current_state: String,
    pub last_run_at: String,
    pub state_path: String,
}

/// Compute the on-disk state path for one deploy. The path
/// is lexically scoped to the project directory, the project
/// id and the deploy identity so two projects (or two
/// deploys on the same project) can never share a state
/// file.
pub fn state_path_for(
    project_dir: &Path,
    project_id: &str,
    identity: &DeployIdentity,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "project id is required to resolve the deploy state path".to_string(),
        });
    }
    if identity.id.trim().is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "deploy id is required to resolve the deploy state path".to_string(),
        });
    }
    Ok(project_dir
        .join(DEPLOY_STATE_DIR)
        .join(project_id)
        .join(&identity.id)
        .join("state.json"))
}

pub fn load_deploy_state(path: &Path) -> Result<DeployState, ForgeError> {
    if !path.exists() {
        return Ok(DeployState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot read deploy state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(DeployState::default());
    }
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::DeployInvalid {
        reason: format!(
            "deploy state at {} is not valid JSON: {err}",
            path.display()
        ),
    })
}

pub fn save_deploy_state(path: &Path, state: &DeployState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::DeployInvalid {
            reason: format!(
                "cannot create deploy state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot serialize deploy state: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot write deploy state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot rename deploy state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Adapter configuration. The CLI/MCP build this from
/// environment variables; Core owns the default values so
/// contract fixtures can stand in for real provider
/// integration.
#[derive(Debug, Clone)]
pub struct DeployAdapterConfig {
    pub deployer_bin: String,
}

impl DeployAdapterConfig {
    pub fn from_env() -> Self {
        let deployer_bin = std::env::var(DEPLOYER_BIN_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DEPLOYER_BIN.to_string());
        DeployAdapterConfig { deployer_bin }
    }
}

/// Redact credential-like substrings from a piece of evidence.
/// The redaction is delegated to
/// [`crate::policy::redact_credentials`] so the deploy,
/// policy and distribution contracts share one definition
/// of "secret".
pub fn redact_deploy_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Capture the current working-tree revision. A non-git
/// project is refused before any deploy side effect runs.
pub fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|err| ForgeError::DeployInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "directory `{}` is not a git working tree with a HEAD commit: {}",
                dir.display(),
                stderr.trim()
            ),
        });
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "git rev-parse returned an empty SHA".to_string(),
        });
    }
    Ok(sha)
}

/// Load an artifact (relative to the project) and return
/// its content hash plus size. The full content is not
/// embedded in the report so the report stays small even
/// for large compose files.
pub fn load_artifact(project_dir: &Path, relative: &str) -> Result<CapturedArtifact, ForgeError> {
    if !lexically_inside(".", relative) {
        return Err(ForgeError::DeployInvalid {
            reason: format!("artifact `{relative}` resolves outside the project"),
        });
    }
    let path = project_dir.join(relative);
    if !path.is_file() {
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "artifact file `{}` does not exist; deploy requires a present artifact",
                path.display()
            ),
        });
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot read artifact {}: {err}", path.display()),
    })?;
    let content_hash = hash_bytes(&bytes);
    Ok(CapturedArtifact {
        path: relative.to_string(),
        content_hash,
        byte_size: bytes.len() as u64,
    })
}

/// Render a [`DeployReport`] for human output. The transport
/// renders the same data the JSON envelope carries so a
/// partial run is observable on stdout.
pub fn render_report_human(report: &DeployReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!("deploy_id: {}", report.identity.id));
    lines.push(format!(
        "target: {} ({})",
        report.target.name, report.target.kind
    ));
    lines.push(format!(
        "source_revision: {}",
        report.identity.source_revision
    ));
    if let Some(artifact) = &report.artifact {
        lines.push(format!("artifact: {}", artifact.path));
        lines.push(format!("artifact_hash: {}", artifact.content_hash));
    }
    if let Some(health) = &report.health {
        lines.push(format!(
            "health: {} ({})",
            health.kind,
            health_label(health)
        ));
    }
    lines.push(format!("adapter: {}", report.adapter));
    lines.push(format!(
        "mode: {}",
        if report.dry_run { "dry-run" } else { "apply" }
    ));
    lines.push(format!("state: {}", report.state_path));
    if !report.stages.is_empty() {
        lines.push("stages:".to_string());
        for outcome in &report.stages {
            lines.push(format!(
                "  - {stage} target=`{target}` {status}: {note}",
                stage = outcome.stage,
                target = outcome.target,
                status = outcome.status,
                note = outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    if let Some(obs) = &report.observation {
        lines.push(format!(
            "observation: {} at {}: {}",
            obs.status, obs.observed_at, obs.detail
        ));
        for line in &obs.evidence {
            lines.push(format!("      evidence: {line}"));
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

fn health_label(health: &DeployHealthSpec) -> String {
    match health.kind.as_str() {
        HEALTH_DOCKER => format!("service={}", health.service.as_deref().unwrap_or("?")),
        HEALTH_HTTP => format!("url={}", health.url.as_deref().unwrap_or("?")),
        HEALTH_PROCESS => format!("process={}", health.process.as_deref().unwrap_or("?")),
        other => other.to_string(),
    }
}

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem. Absolute paths must already sit inside
/// the project, and `..` segments must never escape the
/// root. Symlinks are rechecked canonically at write time.
fn lexically_inside(_project_dir: &str, rel: &str) -> bool {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return false;
    }
    let mut depth = 0i32;
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn meta_with(targets: &str) -> DeploymentMeta {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n{targets}"
            )
            .as_bytes(),
        )
        .expect("manifest");
        manifest.deployment.expect("deployment section")
    }

    #[test]
    fn deploy_config_parses_single_typed_target() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: local\n");
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        assert_eq!(cfg.default_target, "home");
        assert_eq!(cfg.adapter, "local");
        assert_eq!(cfg.targets.len(), 1);
    }

    #[test]
    fn deploy_config_resolves_default_for_multiple_targets() {
        let meta = meta_with(
            "  default: vps\n  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: docker-compose\n",
        );
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        assert_eq!(cfg.default_target, "vps");
        assert_eq!(cfg.adapter, "mixed");
        assert_eq!(cfg.targets.len(), 2);
    }

    #[test]
    fn deploy_config_refuses_multiple_targets_without_default() {
        let meta = meta_with(
            "  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: local\n",
        );
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_unknown_target_kind() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: bogus\n");
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_duplicate_target_name() {
        let meta = meta_with(
            "  default: home\n  targets:\n    - name: home\n      kind: local\n    - name: home\n      kind: docker-compose\n",
        );
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_artifact_outside_project() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: ../etc/passwd\n  targets:\n    - name: home\n      kind: local\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_accepts_legacy_single_target() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  type: docker-compose\n  target: home\n",
        )
        .expect("manifest");
        let cfg = DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap();
        assert_eq!(cfg.default_target, "home");
        assert_eq!(cfg.adapter, "docker-compose");
        assert_eq!(cfg.targets.len(), 1);
    }

    #[test]
    fn deploy_config_refuses_mixing_legacy_and_typed() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  type: docker-compose\n  target: home\n  targets:\n    - name: vps\n      kind: local\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_empty_block() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment: {}\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_identity_is_stable_for_same_inputs() {
        let a = DeployIdentity::derive("app", "home", "deadbeefcafe");
        let b = DeployIdentity::derive("app", "home", "deadbeefcafe");
        assert_eq!(a, b);
        let c = DeployIdentity::derive("app", "vps", "deadbeefcafe");
        assert_ne!(a, c);
        let d = DeployIdentity::derive("app", "home", "different");
        assert_ne!(a, d);
    }

    #[test]
    fn load_artifact_reads_present_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("docker-compose.yml"),
            "services:\n  app:\n    image: app\n",
        )
        .unwrap();
        let captured = load_artifact(tmp.path(), "docker-compose.yml").unwrap();
        assert_eq!(captured.path, "docker-compose.yml");
        assert!(captured.content_hash.len() == 64);
        assert!(captured.byte_size > 0);
    }

    #[test]
    fn load_artifact_refuses_missing_file() {
        let tmp = TempDir::new().unwrap();
        let err = load_artifact(tmp.path(), "docker-compose.yml").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn load_artifact_refuses_outside_project_path() {
        let err = load_artifact(Path::new("."), "../etc/passwd").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn target_lookup_rejects_unknown_name() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: local\n");
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        let err = cfg.target("vps").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn health_spec_requires_kind() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: home\n      kind: local\n  health:\n    service: app\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn health_spec_rejects_unknown_kind() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: bogus\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }
}
