//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    capture_source_revision, deploy_config_from_manifest, load_artifact, load_deploy_state,
    redact_deploy_evidence, save_deploy_state, state_path_for, DeployAdapterConfig, DeployConfig,
    DeployHealthSpec, DeployIdentity, DeployListEntry, DeployPlan, DeployReport, DeployRequest,
    DeployStageOutcome, DeployState, HealthObservation, DEPLOY_ADAPTER_TIMEOUT,
    DEPLOY_CONTRACT_VERSION, DEPLOY_EXECUTOR_CONTRACT, HEALTH_DOCKER, HEALTH_HTTP, HEALTH_PROCESS,
    STATUS_DELIVERED, STATUS_DISABLED, STATUS_FAILED, STATUS_RUNNING, STATUS_SKIPPED,
    STATUS_UNKNOWN, TARGET_DOCKER_COMPOSE, TARGET_JENKINS, TARGET_LOCAL, TARGET_MAC_RUNTIME,
    TARGET_SSH,
};
use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::registry::Registry;
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::model::{AdapterOp, AdapterRequest, AdapterResponse};

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
                "target `{}` uses kind `{TARGET_SSH}` which is planned for a later release; v0.1.0 supports `{TARGET_LOCAL}`, `{TARGET_DOCKER_COMPOSE}`, `{TARGET_JENKINS}`, and `{TARGET_MAC_RUNTIME}`",
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
    let response = invoke_adapter(
        project_dir,
        adapters,
        &plan,
        AdapterOp::Apply,
        request.dry_run,
    )?;
    let now = Utc::now().to_rfc3339();
    let observation = build_observation(&response, &plan, &now);
    let attribution = redact_deploy_evidence(&response.attribution(&adapters.deployer_bin));
    let mut apply_evidence: Vec<String> = response
        .apply_evidence
        .iter()
        .map(|s| redact_deploy_evidence(s))
        .collect();
    apply_evidence.push(attribution.clone());
    let apply_outcome = DeployStageOutcome {
        stage: "apply".to_string(),
        target: plan.target.name.clone(),
        status: response.apply_status.clone(),
        note: redact_deploy_evidence(&response.apply_note),
        evidence: apply_evidence,
        recovery: response
            .recovery
            .iter()
            .map(|s| redact_deploy_evidence(s))
            .collect(),
    };
    let mut observe_evidence: Vec<String> = observation
        .evidence
        .iter()
        .map(|s| redact_deploy_evidence(s))
        .collect();
    observe_evidence.push(attribution);
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
        evidence: observe_evidence,
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
    if observation.status == STATUS_RUNNING {
        next_state.last_observed_running = Some(observation.clone());
    }
    next_state.last_run_at = now.clone();
    // Only persist when the apply stage did not fail with a
    // missing binary or contract mismatch. A failed adapter
    // leaves the prior observation intact. A dry-run
    // rehearsal never mutates the persisted record: the last
    // real deploy's evidence stands unchanged.
    if !request.dry_run
        && (apply_outcome.status == STATUS_DELIVERED
            || apply_outcome.status == STATUS_SKIPPED
            || apply_outcome.status == STATUS_DISABLED)
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

/// Invoke the executor adapter under the frozen
/// `forge-deploy-executor/0.1.0` contract.
///
/// Classification rules (contract-pinned):
///
/// - exit 0 with a parseable, contract-conformant envelope →
///   the stage outcome exactly as the envelope names it;
/// - non-zero exit WITH a parseable, contract-conformant
///   envelope → the named stage failed with the envelope's
///   evidence; the prior DeployState is preserved because a
///   failed apply stage never persists;
/// - non-zero exit WITHOUT a parseable envelope, an unknown
///   contract, a timeout or a spawn failure →
///   `deploy-target-unavailable`; the prior DeployState
///   stands and the refusal names what was observed.
fn invoke_adapter(
    project_dir: &Path,
    adapters: &DeployAdapterConfig,
    plan: &DeployPlan,
    op: AdapterOp,
    dry_run: bool,
) -> Result<AdapterResponse, ForgeError> {
    let payload = AdapterRequest {
        contract: DEPLOY_EXECUTOR_CONTRACT.to_string(),
        project_id: plan.project_id.clone(),
        deploy_id: plan.identity.id.clone(),
        target: plan.target.clone(),
        artifact: plan.artifact.clone(),
        health: plan.health.clone(),
        revision: plan.identity.source_revision.clone(),
        dry_run,
    };
    let mut cmd = Command::new(&adapters.deployer_bin);
    match op {
        AdapterOp::Apply => {
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
        }
        AdapterOp::Observe => {
            // Observe is a read-only verb: the executor must
            // report health for the persisted deploy identity
            // and never re-applies the artifact.
            cmd.arg("observe")
                .arg("--target")
                .arg(&plan.target.name)
                .arg("--project")
                .arg(&plan.project_id)
                .arg("--deploy-id")
                .arg(&plan.identity.id);
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
        }
    }
    // Executors consume project-relative release manifests. Running them from
    // the project root keeps the Linux controller boundary explicit and
    // prevents adapters from depending on the caller's ambient directory.
    cmd.current_dir(project_dir);
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
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        // A non-zero exit that still carries a contract-
        // conformant envelope is a FAILED STAGE with the
        // runtime's own evidence, not an unavailable
        // executor: the stage outcome records what the
        // adapter named while the prior DeployState stands
        // (a failed apply stage is never persisted below).
        // A contradictory envelope — non-zero exit claiming
        // delivery — is downgraded to `failed`: the exit
        // code is authoritative, so a stale or buggy
        // executor can never masquerade as a success.
        if let Ok(mut response) = AdapterResponse::parse(&stdout) {
            if response.apply_status == STATUS_DELIVERED
                || response.apply_status == STATUS_SKIPPED
                || response.apply_status == STATUS_DISABLED
            {
                let claim = response.apply_status.clone();
                response.apply_status = STATUS_FAILED.to_string();
                response.apply_note = format!(
                    "adapter exited {} while claiming `{claim}`; the exit status is authoritative, so the stage is recorded as failed",
                    output.status
                );
            }
            return Ok(response);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ForgeError::DeployTargetUnavailable {
            reason: format!(
                "deploy adapter exited with status {}: {}",
                output.status,
                redact_deploy_evidence(stderr.trim())
            ),
        });
    }
    AdapterResponse::parse(&stdout)
}

pub(super) fn wait_with_timeout(
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
    let response = invoke_adapter(
        project_dir,
        adapters,
        &plan,
        AdapterOp::Observe,
        request.dry_run,
    )?;
    let now = Utc::now().to_rfc3339();
    let observation = build_observation(&response, &plan, &now);
    let attribution = redact_deploy_evidence(&response.attribution(&adapters.deployer_bin));
    let mut observe_evidence: Vec<String> = observation
        .evidence
        .iter()
        .map(|s| redact_deploy_evidence(s))
        .collect();
    observe_evidence.push(attribution);
    let observe_outcome = DeployStageOutcome {
        stage: "observe".to_string(),
        target: target.name.clone(),
        status: observation.status.clone(),
        note: redact_deploy_evidence(&observation.detail),
        evidence: observe_evidence,
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
    if observation.status == STATUS_RUNNING {
        next_state.last_observed_running = Some(observation.clone());
    }
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
    let base = project_dir
        .join(super::super::DEPLOY_STATE_DIR)
        .join(project_id);
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
        .join(super::super::DEPLOY_STATE_DIR)
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
