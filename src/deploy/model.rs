//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{DeploymentHealthMeta, DeploymentMeta};
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::contract::{
    DEFAULT_DEPLOYER_BIN, DEPLOYER_BIN_ENV, HEALTH_DOCKER, HEALTH_HTTP, HEALTH_PROCESS,
    STATUS_FAILED, STATUS_PENDING, STATUS_RUNNING, STATUS_UNKNOWN,
};
use super::prepare::{lexically_inside, validate_target_kind, validate_target_name};
use crate::extension::kinds::{
    default_adapter_for_kind, required_target_fields, supported_target_kinds, DEPLOY_TARGET_KINDS,
};

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
                        "deployment.targets[{idx}] (`{name}`) is missing a `kind`; expected one of {}",
                        supported_target_kinds()
                    ),
                })?;
            validate_target_kind(&kind)?;
            refuse_missing_target_fields(
                &name,
                &kind,
                stated_target_fields(
                    entry.host.as_deref(),
                    entry.user.as_deref(),
                    entry.path.as_deref(),
                    entry.service.as_deref(),
                ),
            )?;
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
                // The legacy form cannot state a host or a remote
                // path, so it cannot express a kind that needs
                // them. It is refused here, in the same words the
                // typed form uses, instead of failing later inside
                // an adapter.
                refuse_missing_target_fields(
                    &name,
                    deploy_type,
                    stated_target_fields(None, None, None, None),
                )?;
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

/// The target fields a target states, by field name. The kind
/// registry names the fields a kind requires; this is the list the
/// requirement is checked against.
fn stated_target_fields<'a>(
    host: Option<&'a str>,
    user: Option<&'a str>,
    path: Option<&'a str>,
    service: Option<&'a str>,
) -> Vec<(&'static str, Option<&'a str>)> {
    vec![
        ("host", host),
        ("user", user),
        ("path", path),
        ("service", service),
    ]
}

/// Refuse a target that does not state the fields its kind
/// requires, naming the kind and the missing fields. The check
/// sits next to the kind check, so a target is validated against
/// its kind in one place, and the requirement itself lives in the
/// registry rather than here.
fn refuse_missing_target_fields(
    name: &str,
    kind: &str,
    stated: Vec<(&'static str, Option<&str>)>,
) -> Result<(), ForgeError> {
    let missing: Vec<&'static str> = required_target_fields(kind)
        .iter()
        .copied()
        .filter(|field| {
            let value = stated
                .iter()
                .find(|(stated_name, _)| stated_name == field)
                .and_then(|(_, value)| *value);
            match value {
                Some(value) => value.trim().is_empty(),
                None => true,
            }
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(ForgeError::DeployInvalid {
        reason: format!(
            "deployment target `{name}` of kind `{kind}` must declare {}",
            render_field_list(&missing)
        ),
    })
}

/// Render a list of field names once, so the refusal's wording
/// cannot drift between one, two and three missing fields.
fn render_field_list(fields: &[&str]) -> String {
    match fields.len() {
        0 => String::new(),
        1 => format!("`{}`", fields[0]),
        2 => format!("`{}` and `{}`", fields[0], fields[1]),
        _ => {
            let head = fields[..fields.len() - 1]
                .iter()
                .map(|field| format!("`{field}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{head} and `{}`", fields[fields.len() - 1])
        }
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
    /// The verdict of the shared delivery rule for this revision.
    /// The plan is only `ready` when the rule is satisfied.
    pub ship_gate: crate::gate::ship_gate::ShipGateStatus,
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
/// The deploy run's verdict.
///
/// The verdict is what the run did. A rehearsal is a plan, and a plan
/// is not a delivery: a rehearsal contacts the adapter, but it deploys
/// nothing and observes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeployVerdict {
    /// The deploy ran and every stage completed.
    Delivered,
    /// No deploy was attempted. The report is a rehearsal.
    Planned,
    /// A stage did not complete.
    Failed,
    /// A plan the shared delivery rule blocks. A plan never reaches an
    /// adapter, so this arm belongs to the plan lane alone: the report
    /// of an apply never carries it.
    Blocked,
}
impl DeployVerdict {
    /// The stable label the report, the JSON document and the journal
    /// detail publish.
    pub fn label(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "delivered",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
            DeployVerdict::Blocked => "blocked",
        }
    }
    /// The registry journal state this verdict records. A delivery is a
    /// completed deploy, a rehearsal is a planned deploy, a failure is
    /// a failed deploy and a blocked plan is a blocked deploy.
    pub fn journal_state(self) -> &'static str {
        match self {
            DeployVerdict::Delivered => "done",
            DeployVerdict::Planned => "planned",
            DeployVerdict::Failed => "failed",
            DeployVerdict::Blocked => "blocked",
        }
    }
}
/// The one rule that judges a deploy run.
///
/// A rehearsal attempted no deploy, whatever its stages say: the
/// adapter rehearsed, so its stages are the adapter's rehearsal
/// output and not evidence that anything was deployed. A run that
/// attempted a deploy is a delivery only when every stage completed.
pub fn verdict_for_report(dry_run: bool, healthy: bool) -> DeployVerdict {
    if dry_run {
        return DeployVerdict::Planned;
    }
    if healthy {
        DeployVerdict::Delivered
    } else {
        DeployVerdict::Failed
    }
}
/// The one rule that judges a deploy plan.
///
/// A plan deploys nothing: it names the target, the artifact and the
/// revision, and asks the shared delivery rule whether the revision may
/// go out. A plan the rule allows is a planned run and a plan the rule
/// blocks is a blocked run. Neither is a completed deployment.
pub fn verdict_for_plan(ready: bool) -> DeployVerdict {
    if ready {
        DeployVerdict::Planned
    } else {
        DeployVerdict::Blocked
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
    /// The run's verdict. The field is always present, so a reader
    /// learns what the run did without re-deriving it from the stages.
    pub verdict: DeployVerdict,
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
#[derive(Debug, Clone, Default)]
pub struct DeployAdapterConfig {
    /// The shared adapter binary. Empty when the shared variable is
    /// unset, so the resolution can tell "unset" from "set to the
    /// default".
    pub deployer_bin: String,
    /// The adapter binary each kind named for itself.
    pub kind_bins: BTreeMap<String, String>,
}
impl DeployAdapterConfig {
    pub fn from_env() -> Self {
        let deployer_bin = std::env::var(DEPLOYER_BIN_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_default();
        let mut kind_bins: BTreeMap<String, String> = BTreeMap::new();
        for entry in DEPLOY_TARGET_KINDS {
            let Some(variable) = entry.adapter_env else {
                continue;
            };
            if let Ok(value) = std::env::var(variable) {
                if !value.trim().is_empty() {
                    kind_bins.insert(entry.kind.to_string(), value);
                }
            }
        }
        DeployAdapterConfig {
            deployer_bin,
            kind_bins,
        }
    }
    /// The adapter binary that serves one kind, in one order: the
    /// kind's own binary, then the shared binary, then the kind's
    /// default. A kind that names no adapter of its own resolves to
    /// the shared binary, exactly as it did before the resolution
    /// became per kind.
    pub fn bin_for_kind(&self, kind: &str) -> String {
        if let Some(bin) = self.kind_bins.get(kind) {
            return bin.clone();
        }
        if !self.deployer_bin.trim().is_empty() {
            return self.deployer_bin.clone();
        }
        default_adapter_for_kind(kind)
            .map(|bin| bin.to_string())
            .unwrap_or_else(|| DEFAULT_DEPLOYER_BIN.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::Manifest;
    use crate::deploy::deploy_config_from_manifest;

    /// A manifest naming one deploy target kind and the fields it
    /// states.
    fn manifest_with(body: &str) -> Manifest {
        Manifest::parse(
            std::path::Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  default: vps\n{body}"
            )
            .as_bytes(),
        )
        .expect("manifest")
    }

    #[test]
    fn the_adapter_is_resolved_from_the_kind_then_the_shared_binary_then_the_default() {
        let own = DeployAdapterConfig {
            deployer_bin: "/usr/local/bin/forge-deployer".to_string(),
            kind_bins: BTreeMap::from([(
                "ssh".to_string(),
                "/usr/local/bin/forge-deployer-ssh".to_string(),
            )]),
        };
        assert_eq!(
            own.bin_for_kind("ssh"),
            "/usr/local/bin/forge-deployer-ssh",
            "the kind's own binary wins"
        );
        assert_eq!(
            own.bin_for_kind("local"),
            "/usr/local/bin/forge-deployer",
            "a kind that names no binary uses the shared binary"
        );

        let shared = DeployAdapterConfig {
            deployer_bin: "/usr/local/bin/forge-deployer".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shared.bin_for_kind("ssh"),
            "/usr/local/bin/forge-deployer",
            "the shared binary serves a kind that names no binary of its own"
        );

        let neither = DeployAdapterConfig::default();
        assert_eq!(
            neither.bin_for_kind("ssh"),
            "forge-deployer-ssh",
            "the kind's default is the last rung"
        );
        assert_eq!(
            neither.bin_for_kind("local"),
            "forge-deployer",
            "a kind without its own binary keeps the shared default"
        );
        assert_eq!(
            neither.bin_for_kind("carrier-pigeon"),
            DEFAULT_DEPLOYER_BIN,
            "an unknown kind falls back to the shared default"
        );
    }

    #[test]
    fn the_kinds_without_their_own_adapter_resolve_exactly_as_before() {
        let configured = DeployAdapterConfig {
            deployer_bin: "/usr/local/bin/forge-deployer".to_string(),
            ..Default::default()
        };
        for kind in ["local", "docker-compose", "jenkins", "mac-runtime"] {
            assert_eq!(
                configured.bin_for_kind(kind),
                configured.deployer_bin,
                "the kind `{kind}` must resolve to the shared binary"
            );
        }
        // The registry's default for the shared kinds is the shared
        // default constant, so the two cannot drift.
        for kind in ["local", "docker-compose", "jenkins", "mac-runtime"] {
            assert_eq!(
                DeployAdapterConfig::default().bin_for_kind(kind),
                DEFAULT_DEPLOYER_BIN,
                "the registry's default for `{kind}` must be the shared default"
            );
        }
    }

    #[test]
    fn a_target_that_states_its_kind_s_fields_is_accepted() {
        let manifest = manifest_with(
            "  targets:\n    - name: vps\n      kind: ssh\n      host: home-server-01\n      path: /srv/app/docker-compose.yml\n",
        );
        let config = deploy_config_from_manifest(&manifest).expect("an ssh target with its fields");
        assert_eq!(config.targets.len(), 1);
        assert_eq!(config.targets[0].host.as_deref(), Some("home-server-01"));
        assert_eq!(
            config.targets[0].path.as_deref(),
            Some("/srv/app/docker-compose.yml")
        );
    }

    #[test]
    fn a_target_that_does_not_state_its_kind_s_fields_is_refused_by_name() {
        let manifest = manifest_with(
            "  targets:\n    - name: vps\n      kind: ssh\n      host: home-server-01\n",
        );
        let err =
            deploy_config_from_manifest(&manifest).expect_err("a target without a path is refused");
        assert_eq!(err.code(), "deploy-invalid");
        assert!(
            err.to_string()
                .contains("deployment target `vps` of kind `ssh` must declare `path`"),
            "the refusal names the kind and the missing field: {err}"
        );

        let manifest = manifest_with(
            "  targets:\n    - name: vps\n      kind: ssh\n      path: /srv/app/docker-compose.yml\n",
        );
        let err =
            deploy_config_from_manifest(&manifest).expect_err("a target without a host is refused");
        assert!(
            err.to_string()
                .contains("deployment target `vps` of kind `ssh` must declare `host`"),
            "the refusal names the kind and the missing field: {err}"
        );

        let manifest = manifest_with("  targets:\n    - name: vps\n      kind: ssh\n");
        let err = deploy_config_from_manifest(&manifest)
            .expect_err("a target without a host and a path is refused");
        assert!(
            err.to_string()
                .contains("deployment target `vps` of kind `ssh` must declare `host` and `path`"),
            "the refusal names the kind and both missing fields: {err}"
        );
    }

    #[test]
    fn the_legacy_type_form_cannot_state_an_ssh_target() {
        let manifest = manifest_with("  type: ssh\n  target: vps\n");
        let err = deploy_config_from_manifest(&manifest)
            .expect_err("the legacy form cannot state a host and a path");
        assert_eq!(err.code(), "deploy-invalid");
        assert!(
            err.to_string()
                .contains("deployment target `vps` of kind `ssh` must declare `host` and `path`"),
            "the legacy form is refused in the same words: {err}"
        );
    }

    #[test]
    fn a_blank_field_counts_as_missing() {
        let manifest = manifest_with(
            "  targets:\n    - name: vps\n      kind: ssh\n      host: \"  \"\n      path: /srv/app/docker-compose.yml\n",
        );
        let err = deploy_config_from_manifest(&manifest).expect_err("a blank host is refused");
        assert!(
            err.to_string()
                .contains("deployment target `vps` of kind `ssh` must declare `host`"),
            "a blank field is a missing field: {err}"
        );
    }

    #[test]
    fn a_rehearsal_is_a_plan_and_never_a_delivery() {
        // The one rule: a rehearsal deployed nothing, whatever the
        // adapter's rehearsal said.
        assert_eq!(
            verdict_for_report(true, true),
            DeployVerdict::Planned,
            "a rehearsal is a plan even when every stage completed"
        );
        assert_eq!(
            verdict_for_report(true, false),
            DeployVerdict::Planned,
            "a rehearsal is a plan even when a stage failed"
        );
    }

    #[test]
    fn a_complete_run_is_a_delivery_and_a_broken_run_is_a_failure() {
        assert_eq!(verdict_for_report(false, true), DeployVerdict::Delivered);
        assert_eq!(verdict_for_report(false, false), DeployVerdict::Failed);
    }

    #[test]
    fn the_verdict_publishes_a_label_and_a_journal_state() {
        assert_eq!(DeployVerdict::Delivered.label(), "delivered");
        assert_eq!(DeployVerdict::Planned.label(), "planned");
        assert_eq!(DeployVerdict::Failed.label(), "failed");
        assert_eq!(DeployVerdict::Blocked.label(), "blocked");
        assert_eq!(DeployVerdict::Delivered.journal_state(), "done");
        assert_eq!(DeployVerdict::Planned.journal_state(), "planned");
        assert_eq!(DeployVerdict::Failed.journal_state(), "failed");
        assert_eq!(DeployVerdict::Blocked.journal_state(), "blocked");
    }

    #[test]
    fn a_plan_is_never_a_completed_deployment() {
        // The one rule for the plan lane: a plan deploys nothing, so
        // it is a planned run and never a delivery.
        assert_eq!(
            verdict_for_plan(true),
            DeployVerdict::Planned,
            "a plan the delivery rule allows is a planned run"
        );
        assert_eq!(
            verdict_for_plan(false),
            DeployVerdict::Blocked,
            "a plan the delivery rule blocks is a blocked run"
        );
        assert_ne!(
            verdict_for_plan(true).journal_state(),
            "done",
            "a plan is never recorded as a completed deployment"
        );
        assert_ne!(
            verdict_for_plan(false).journal_state(),
            "done",
            "a blocked plan is never recorded as a completed deployment"
        );
    }
}
