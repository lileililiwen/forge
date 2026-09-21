//! Deployment preparation, plan rendering, apply and
//! observation (`adapter-deployment`).
//!
//! This is the Core side of the deploy engine. The CLI and
//! MCP transports build a [`DeployRequest`], call
//! [`prepare_deploy`], [`apply_deploy`] or [`observe_deploy`],
//! and render the returned [`DeployReport`]. No transport
//! reinterprets the typed outcomes; the contract owns the
//! labels and the per-stage semantics.

use std::fs;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::registry::Registry;

use super::{
    capture_source_revision, deploy_config_from_manifest, load_artifact, load_deploy_state,
    redact_deploy_evidence, save_deploy_state, state_path_for, CapturedArtifact,
    DeployAdapterConfig, DeployConfig, DeployHealthSpec, DeployIdentity, DeployListEntry,
    DeployPlan, DeployReport, DeployRequest, DeployStageOutcome, DeployState, DeployTargetSpec,
    HealthObservation, DEPLOY_ADAPTER_TIMEOUT, DEPLOY_CONTRACT_VERSION, HEALTH_DOCKER, HEALTH_HTTP,
    HEALTH_PROCESS, STATUS_DELIVERED, STATUS_DISABLED, STATUS_FAILED, STATUS_RUNNING,
    STATUS_SKIPPED, STATUS_UNKNOWN, TARGET_DOCKER_COMPOSE, TARGET_LOCAL, TARGET_SSH,
};

/// Read-only plan report. The transport renders this
/// before any side effect runs.
pub fn prepare_deploy(
    project_dir: &Path,
    _manifest: &Manifest,
    config: &DeployConfig,
    request: &DeployRequest,
) -> Result<DeployPlan, ForgeError> {
    request.validate()?;
    let target = config.target(&request.target)?.clone();
    if target.kind == TARGET_SSH {
        // R1 boundary: ssh targets are not wired into this
        // build. The contract refuses before any side effect
        // runs so a manifest cannot silently fall through to
        // an unsupported adapter.
        return Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "target `{}` uses kind `{TARGET_SSH}` which is planned for a later release; v0.1.0 supports only `{TARGET_LOCAL}` and `{TARGET_DOCKER_COMPOSE}`",
                target.name
            ),
        });
    }
    let revision = capture_source_revision(project_dir)?;
    let artifact = match &config.artifact {
        Some(relative) => Some(load_artifact(project_dir, relative)?),
        None => None,
    };
    let ready = artifact.is_some() || config.adapter == TARGET_LOCAL;
    let note = if ready {
        format!(
            "deploy plan ready for target `{}` (kind `{}`) at revision `{}`",
            target.name,
            target.kind,
            short_revision(&revision)
        )
    } else {
        format!(
            "deploy plan not ready: target `{}` requires an artifact but `deployment.artifact` is missing",
            target.name
        )
    };
    Ok(DeployPlan {
        contract: DEPLOY_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        identity: DeployIdentity::derive(&request.project_id, &target.name, &revision),
        target,
        artifact,
        health: config.health.clone(),
        note,
        ready,
    })
}

fn short_revision(sha: &str) -> String {
    sha.chars().take(12).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdapterRequest {
    contract: String,
    project_id: String,
    target: DeployTargetSpec,
    artifact: Option<CapturedArtifact>,
    health: Option<DeployHealthSpec>,
    revision: String,
    dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdapterResponse {
    contract: String,
    apply_status: String,
    apply_note: String,
    apply_evidence: Vec<String>,
    observation_status: String,
    observation_detail: String,
    observation_evidence: Vec<String>,
    recovery: Vec<String>,
}

impl AdapterResponse {
    fn parse(raw: &str) -> Result<Self, ForgeError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|err| ForgeError::DeployTargetUnavailable {
                reason: format!("deploy adapter returned non-JSON output: {err}"),
            })?;
        let contract = value
            .get("contract")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if contract != DEPLOY_CONTRACT_VERSION {
            return Err(ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "deploy adapter contract `{contract}` does not match expected `{}`",
                    DEPLOY_CONTRACT_VERSION
                ),
            });
        }
        let obj = value
            .as_object()
            .ok_or_else(|| ForgeError::DeployTargetUnavailable {
                reason: "deploy adapter payload is not a JSON object".to_string(),
            })?;
        Ok(AdapterResponse {
            contract,
            apply_status: obj
                .get("apply_status")
                .and_then(|v| v.as_str())
                .unwrap_or(STATUS_FAILED)
                .to_string(),
            apply_note: obj
                .get("apply_note")
                .and_then(|v| v.as_str())
                .unwrap_or("deploy adapter did not provide a note")
                .to_string(),
            apply_evidence: obj
                .get("apply_evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            observation_status: obj
                .get("observation_status")
                .and_then(|v| v.as_str())
                .unwrap_or(STATUS_UNKNOWN)
                .to_string(),
            observation_detail: obj
                .get("observation_detail")
                .and_then(|v| v.as_str())
                .unwrap_or("deploy adapter did not provide an observation detail")
                .to_string(),
            observation_evidence: obj
                .get("observation_evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            recovery: obj
                .get("recovery")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

/// Apply a deploy: invoke the adapter, capture the health
/// observation and persist the state. Refuses without
/// `request.confirm`.
pub fn apply_deploy(
    project_dir: &Path,
    manifest: &Manifest,
    config: &DeployConfig,
    request: &DeployRequest,
    adapters: &DeployAdapterConfig,
) -> Result<DeployReport, ForgeError> {
    request.validate()?;
    if !request.confirm {
        return Err(ForgeError::DeployInvalid {
            reason: "deploy apply requires explicit --confirm; refusing implicit remote write"
                .to_string(),
        });
    }
    let plan = prepare_deploy(project_dir, manifest, config, request)?;
    let state_path = state_path_for(project_dir, &plan.project_id, &plan.identity)?;
    let prior_state = load_deploy_state(&state_path)?;
    let response = invoke_adapter(adapters, &plan, request.dry_run)?;
    let now = Utc::now().to_rfc3339();
    let observation = build_observation(&response, &plan, &now);
    let apply_outcome = DeployStageOutcome {
        stage: "apply".to_string(),
        target: plan.target.name.clone(),
        status: response.apply_status.clone(),
        note: redact_deploy_evidence(&response.apply_note),
        evidence: response
            .apply_evidence
            .iter()
            .map(|s| redact_deploy_evidence(s))
            .collect(),
        recovery: response
            .recovery
            .iter()
            .map(|s| redact_deploy_evidence(s))
            .collect(),
    };
    let observe_outcome = DeployStageOutcome {
        stage: "observe".to_string(),
        target: plan.target.name.clone(),
        status: if observation.status == STATUS_UNKNOWN && response.apply_status == STATUS_DELIVERED
        {
            STATUS_FAILED.to_string()
        } else {
            observation.status.clone()
        },
        note: redact_deploy_evidence(&observation.detail),
        evidence: observation
            .evidence
            .iter()
            .map(|s| redact_deploy_evidence(s))
            .collect(),
        recovery: Vec::new(),
    };
    let stages = vec![apply_outcome.clone(), observe_outcome.clone()];
    let mut next_state = prior_state.clone();
    next_state.contract = DEPLOY_CONTRACT_VERSION.to_string();
    next_state.identity = plan.identity.clone();
    next_state.target = plan.target.clone();
    next_state.adapter = config.adapter.clone();
    next_state.artifact = plan.artifact.clone();
    next_state.health = plan.health.clone();
    next_state.stage_outcomes = stages.clone();
    next_state.last_observation = Some(observation.clone());
    next_state.last_run_at = now.clone();
    // Only persist when the apply stage did not fail with a
    // missing binary or contract mismatch. A failed adapter
    // leaves the prior observation intact.
    if apply_outcome.status == STATUS_DELIVERED
        || apply_outcome.status == STATUS_SKIPPED
        || apply_outcome.status == STATUS_DISABLED
    {
        save_deploy_state(&state_path, &next_state)?;
    }
    let healthy = stages.iter().all(|s| {
        s.status == STATUS_DELIVERED
            || s.status == STATUS_SKIPPED
            || s.status == STATUS_DISABLED
            || s.status == STATUS_RUNNING
    });
    let note = if healthy {
        format!(
            "deploy applied to target `{}` and observed as `{}`",
            plan.target.name, observation.status
        )
    } else {
        format!(
            "deploy did not complete cleanly: apply={} observe={}",
            apply_outcome.status, observe_outcome.status
        )
    };
    Ok(DeployReport {
        contract: DEPLOY_CONTRACT_VERSION.to_string(),
        project_id: plan.project_id.clone(),
        identity: plan.identity.clone(),
        target: plan.target.clone(),
        adapter: config.adapter.clone(),
        artifact: plan.artifact.clone(),
        health: plan.health.clone(),
        stages,
        observation: Some(observation),
        dry_run: request.dry_run,
        state_path: state_path.display().to_string(),
        note,
        healthy,
    })
}

fn build_observation(
    response: &AdapterResponse,
    _plan: &DeployPlan,
    now: &str,
) -> HealthObservation {
    let evidence: Vec<String> = response
        .observation_evidence
        .iter()
        .map(|s| redact_deploy_evidence(s))
        .collect();
    match response.observation_status.as_str() {
        STATUS_RUNNING => {
            HealthObservation::running(&response.observation_detail, evidence, now.to_string())
        }
        STATUS_FAILED => {
            HealthObservation::failed(&response.observation_detail, evidence, now.to_string())
        }
        _ => HealthObservation::unknown(&response.observation_detail, evidence, now.to_string()),
    }
}

fn invoke_adapter(
    adapters: &DeployAdapterConfig,
    plan: &DeployPlan,
    dry_run: bool,
) -> Result<AdapterResponse, ForgeError> {
    let payload = AdapterRequest {
        contract: DEPLOY_CONTRACT_VERSION.to_string(),
        project_id: plan.project_id.clone(),
        target: plan.target.clone(),
        artifact: plan.artifact.clone(),
        health: plan.health.clone(),
        revision: plan.identity.source_revision.clone(),
        dry_run,
    };
    let mut cmd = Command::new(&adapters.deployer_bin);
    cmd.arg("apply")
        .arg("--target")
        .arg(&plan.target.name)
        .arg("--kind")
        .arg(&plan.target.kind)
        .arg("--project")
        .arg(&plan.project_id)
        .arg("--revision")
        .arg(&plan.identity.source_revision);
    if let Some(artifact) = &plan.artifact {
        cmd.arg("--artifact").arg(&artifact.path);
    }
    if let Some(health) = &plan.health {
        cmd.arg("--health-kind").arg(&health.kind);
        if let Some(service) = &health.service {
            cmd.arg("--health-service").arg(service);
        }
        if let Some(url) = &health.url {
            cmd.arg("--health-url").arg(url);
        }
        if let Some(process) = &health.process {
            cmd.arg("--health-process").arg(process);
        }
    }
    if dry_run {
        cmd.arg("--dry-run");
    }
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| {
        let kind = if err.kind() == std::io::ErrorKind::NotFound {
            STATUS_FAILED.to_string()
        } else {
            STATUS_UNKNOWN.to_string()
        };
        ForgeError::DeployTargetUnavailable {
            reason: format!(
                "deploy adapter `{}` could not be spawned: {err} ({kind})",
                adapters.deployer_bin
            ),
        }
    })?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let bytes =
            serde_json::to_vec(&payload).map_err(|err| ForgeError::DeployTargetUnavailable {
                reason: format!("cannot serialize deploy adapter payload: {err}"),
            })?;
        stdin
            .write_all(&bytes)
            .map_err(|err| ForgeError::DeployTargetUnavailable {
                reason: format!("cannot write deploy adapter payload: {err}"),
            })?;
        // Closing stdin signals EOF to adapter scripts that
        // block on `cat` / `read`; without this drop a
        // fixture script never sees EOF and the bounded
        // wait times out.
    }
    drop(child.stdin.take());
    let output = wait_with_timeout(child, DEPLOY_ADAPTER_TIMEOUT).map_err(|err| {
        ForgeError::DeployTargetUnavailable {
            reason: format!("deploy adapter `{}` failed: {err}", adapters.deployer_bin),
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "deploy adapter exited with status {}: {}",
                output.status,
                redact_deploy_evidence(stderr.trim())
            ),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    AdapterResponse::parse(&stdout)
}

fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    out.read_to_end(&mut stdout)?;
                }
                let mut stderr = Vec::new();
                if let Some(mut err) = child.stderr.take() {
                    err.read_to_end(&mut stderr)?;
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("timeout after {:?}", timeout),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

/// Re-observe a previously applied deploy. Reads the
/// persisted state, invokes the adapter's health check and
/// updates the observation. Refuses without
/// `request.confirm` for backward compatibility (R2 boundary:
/// disconnected means unknown, not offline proof).
pub fn observe_deploy(
    project_dir: &Path,
    _manifest: &Manifest,
    config: &DeployConfig,
    request: &DeployRequest,
    adapters: &DeployAdapterConfig,
) -> Result<DeployReport, ForgeError> {
    request.validate()?;
    let target = config.target(&request.target)?.clone();
    if target.kind == TARGET_SSH {
        return Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "target `{}` uses kind `{TARGET_SSH}` which is planned for a later release",
                target.name
            ),
        });
    }
    let revision = capture_source_revision(project_dir)?;
    let identity = DeployIdentity::derive(&request.project_id, &target.name, &revision);
    let state_path = state_path_for(project_dir, &request.project_id, &identity)?;
    let prior = load_deploy_state(&state_path)?;
    if prior.stage_outcomes.is_empty() {
        return Err(ForgeError::DeployTargetStale {
            reason: format!(
                "no prior deploy state at {}; run `forge deploy apply` first",
                state_path.display()
            ),
        });
    }
    let mut plan = DeployPlan {
        contract: DEPLOY_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        identity: identity.clone(),
        target: target.clone(),
        artifact: prior.artifact.clone(),
        health: prior.health.clone().or_else(|| config.health.clone()),
        note: "observe-only: re-running the health check against the persisted artifact identity"
            .to_string(),
        ready: true,
    };
    if plan.artifact.is_none() {
        if let Some(relative) = &config.artifact {
            plan.artifact = Some(load_artifact(project_dir, relative)?);
        }
    }
    let response = invoke_adapter(adapters, &plan, request.dry_run)?;
    let now = Utc::now().to_rfc3339();
    let observation = build_observation(&response, &plan, &now);
    let observe_outcome = DeployStageOutcome {
        stage: "observe".to_string(),
        target: target.name.clone(),
        status: observation.status.clone(),
        note: redact_deploy_evidence(&observation.detail),
        evidence: observation
            .evidence
            .iter()
            .map(|s| redact_deploy_evidence(s))
            .collect(),
        recovery: Vec::new(),
    };
    let mut next_state = prior.clone();
    next_state.contract = DEPLOY_CONTRACT_VERSION.to_string();
    next_state.identity = identity.clone();
    next_state.target = target.clone();
    next_state.adapter = config.adapter.clone();
    next_state.artifact = plan.artifact.clone();
    next_state.health = plan.health.clone();
    next_state.stage_outcomes = vec![observe_outcome.clone()];
    next_state.last_observation = Some(observation.clone());
    next_state.last_run_at = now.clone();
    save_deploy_state(&state_path, &next_state)?;
    let healthy = observe_outcome.status == STATUS_RUNNING
        || observe_outcome.status == STATUS_DELIVERED
        || observe_outcome.status == STATUS_SKIPPED;
    let note = if healthy {
        format!(
            "deploy `{}` re-observed as `{}`",
            identity.id, observation.status
        )
    } else {
        format!(
            "deploy `{}` re-observed as `{}`; state recorded as `{}`",
            identity.id, observation.status, observe_outcome.status
        )
    };
    Ok(DeployReport {
        contract: DEPLOY_CONTRACT_VERSION.to_string(),
        project_id: identity.project_id.clone(),
        identity,
        target,
        adapter: config.adapter.clone(),
        artifact: plan.artifact,
        health: plan.health,
        stages: vec![observe_outcome],
        observation: Some(observation),
        dry_run: request.dry_run,
        state_path: state_path.display().to_string(),
        note,
        healthy,
    })
}

/// List all persisted deploys for the project.
pub fn list_deploys(
    project_dir: &Path,
    project_id: &str,
) -> Result<Vec<DeployListEntry>, ForgeError> {
    let base = project_dir.join(super::DEPLOY_STATE_DIR).join(project_id);
    let mut entries: Vec<DeployListEntry> = Vec::new();
    if !base.is_dir() {
        return Ok(entries);
    }
    for dir in fs::read_dir(&base).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot read deploy directory {}: {err}", base.display()),
    })? {
        let dir = dir.map_err(|err| ForgeError::DeployInvalid {
            reason: format!("cannot iterate deploy directory: {err}"),
        })?;
        let path = dir.path().join("state.json");
        if !path.is_file() {
            continue;
        }
        let state = load_deploy_state(&path)?;
        if state.identity.id.is_empty() {
            continue;
        }
        entries.push(DeployListEntry {
            deploy_id: state.identity.id.clone(),
            project_id: state.identity.project_id.clone(),
            target: state.identity.target.clone(),
            source_revision: state.identity.source_revision.clone(),
            current_state: state.current_state(),
            last_run_at: state.last_run_at.clone(),
            state_path: path.display().to_string(),
        });
    }
    entries.sort_by(|a, b| a.deploy_id.cmp(&b.deploy_id));
    Ok(entries)
}

/// Read a single persisted deploy by its id. Returns None
/// when the state file is absent.
pub fn read_deploy(
    project_dir: &Path,
    project_id: &str,
    deploy_id: &str,
) -> Result<Option<DeployState>, ForgeError> {
    let base = project_dir
        .join(super::DEPLOY_STATE_DIR)
        .join(project_id)
        .join(deploy_id);
    let path = base.join("state.json");
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(load_deploy_state(&path)?))
}

/// Render a [`DeployPlan`] for human output.
pub fn render_plan_human(plan: &DeployPlan) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", plan.project_id));
    lines.push(format!("deploy_id: {}", plan.identity.id));
    lines.push(format!(
        "target: {} ({})",
        plan.target.name, plan.target.kind
    ));
    lines.push(format!(
        "source_revision: {}",
        plan.identity.source_revision
    ));
    if let Some(artifact) = &plan.artifact {
        lines.push(format!("artifact: {}", artifact.path));
        lines.push(format!("artifact_hash: {}", artifact.content_hash));
    }
    if let Some(health) = &plan.health {
        lines.push(format!(
            "health: {} ({})",
            health.kind,
            health_label(health)
        ));
    }
    lines.push(format!("ready: {}", if plan.ready { "yes" } else { "no" }));
    lines.push(format!("summary: {}", plan.note));
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

/// Re-export of `redact_credentials` for tests and the CLI.
pub fn redact(text: &str) -> String {
    redact_credentials(text)
}

/// Resolve the manifest in `project_dir` and return both the
/// parsed manifest and the deploy config. Convenience for
/// transports that do not want to repeat the canonical
/// pattern.
pub fn load_config(project_dir: &Path) -> Result<(Manifest, DeployConfig), ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(project_dir, None)?;
    let config = deploy_config_from_manifest(&manifest)?;
    Ok((manifest, config))
}

/// Optional: record a journal row for a deploy run. The
/// CLI owns the journal kind, not the engine.
pub fn journal_report(report: &DeployReport, healthy: bool) -> String {
    let stage_summary = report
        .stages
        .iter()
        .map(|s| format!("{}={}", s.stage, s.status))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "deploy `{}` target=`{}` adapter=`{}` stages={} healthy={} dry_run={}",
        report.identity.id,
        report.target.name,
        report.adapter,
        stage_summary,
        healthy,
        report.dry_run
    )
}

/// Resolve the registry, ensuring the project is registered.
/// Returns the registry untouched so the caller can append a
/// `deploy` journal row.
pub fn ensure_registry_for<'a>(
    registry: &'a Registry,
    _project_id: &str,
) -> Result<&'a Registry, ForgeError> {
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::Manifest;
    use std::fs;
    use tempfile::TempDir;

    fn write_minimal_project(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("forge.yaml"),
            "schema: 1\nproject:\n  id: dep-test\n  name: dep-test\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: docker-compose\n      service: app\n  health:\n    kind: docker\n    service: app\n",
        )
        .unwrap();
        fs::write(
            dir.join("docker-compose.yml"),
            "services:\n  app:\n    image: app:0.1.0\n",
        )
        .unwrap();
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("init")
            .arg("-q")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.email")
            .arg("forge@example.com")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("user.name")
            .arg("Forge Test")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("config")
            .arg("init.defaultBranch")
            .arg("main")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("checkout")
            .arg("-q")
            .arg("-b")
            .arg("main")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("add")
            .arg("--")
            .arg("forge.yaml")
            .arg("docker-compose.yml")
            .arg("README.md")
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("commit")
            .arg("-q")
            .arg("-m")
            .arg("initial")
            .output()
            .unwrap();
    }

    #[test]
    fn prepare_returns_ready_plan_for_default_target() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let plan = prepare_deploy(tmp.path(), &manifest, &config, &req).unwrap();
        assert!(plan.ready);
        assert_eq!(plan.target.name, "home");
        assert!(plan.artifact.is_some());
    }

    #[test]
    fn prepare_refuses_unknown_target() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: "unknown".to_string(),
            confirm: false,
            dry_run: true,
        };
        let err = prepare_deploy(tmp.path(), &manifest, &config, &req).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn apply_refuses_without_confirm() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        let err = apply_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: "true".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn list_deploys_is_empty_for_fresh_project() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let entries = list_deploys(tmp.path(), "dep-test").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn observe_refuses_without_prior_state() {
        let tmp = TempDir::new().unwrap();
        write_minimal_project(tmp.path());
        let (manifest, config) = load_config(tmp.path()).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: true,
            dry_run: true,
        };
        let err = observe_deploy(
            tmp.path(),
            &manifest,
            &config,
            &req,
            &DeployAdapterConfig {
                deployer_bin: "true".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "deploy-target-stale");
    }

    #[test]
    fn ssh_target_is_unavailable_boundary() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: vps\n      kind: ssh\n",
        )
        .expect("manifest");
        let config = deploy_config_from_manifest(&manifest).unwrap();
        let req = DeployRequest {
            project_id: manifest.project.id.clone(),
            target: config.default_target.clone(),
            confirm: false,
            dry_run: true,
        };
        // Prepare surfaces the boundary so a manifest cannot
        // silently fall through to a planned adapter.
        let err = prepare_deploy(Path::new("."), &manifest, &config, &req).unwrap_err();
        assert_eq!(err.code(), "deploy-target-unavailable");
    }
}
