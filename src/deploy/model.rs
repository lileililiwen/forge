//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{DeploymentHealthMeta, DeploymentMeta};
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::contract::{
    DEFAULT_DEPLOYER_BIN, DEPLOYER_BIN_ENV, HEALTH_DOCKER, HEALTH_HTTP, HEALTH_PROCESS,
    STATUS_FAILED, STATUS_PENDING, STATUS_RUNNING, STATUS_UNKNOWN, TARGET_DOCKER_COMPOSE,
    TARGET_JENKINS, TARGET_LOCAL, TARGET_MAC_RUNTIME,
};
use super::prepare::{lexically_inside, validate_target_kind, validate_target_name};

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
            let kind = entry
                .kind
                .clone()
                .ok_or_else(|| ForgeError::DeployInvalid {
                    reason: format!(
                        "deployment.targets[{idx}] (`{name}`) is missing a `kind`; expected one of `{TARGET_LOCAL}`, `{TARGET_DOCKER_COMPOSE}`, `{TARGET_JENKINS}`, `{TARGET_MAC_RUNTIME}`"
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
                reason: "deployment block mixes typed `targets[]` with legacy `type`/`target`; declare one or the other"
                    .to_string(),
            });
        }
        if typed_targets.is_empty() {
            return Err(ForgeError::DeployInvalid {
                reason: "deployment block declares no targets; declare at least one `deployment.targets[]` entry or a legacy `type`"
                    .to_string(),
            });
        }
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
                reason: "deploy target must not be empty; pass `--target <name>` or set `deployment.default`"
                    .to_string(),
            });
        }
        Ok(())
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployHealthSpec {
    pub kind: String,
    pub service: Option<String>,
    pub url: Option<String>,
    pub process: Option<String>,
    pub interval_seconds: Option<u32>,
}
impl DeployHealthSpec {
    pub(super) fn from_manifest_meta(meta: &DeploymentHealthMeta) -> Result<Self, ForgeError> {
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
