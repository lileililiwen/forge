//! Release preparation, plan rendering and apply logic.
//!
//! This is the Core side of the release engine. The CLI and
//! MCP transports build a [`ReleaseRequest`], call
//! [`prepare_release`] or [`apply_release`], and render the
//! returned [`ReleaseReport`]. No transport reinterprets the
//! typed outcomes; the contract owns the labels and the
//! per-stage semantics.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror as plan_distribution,
    DistributionConfig, MirrorRequest,
};
use crate::gitops::{commit_paths, push_ref, run_test};
use crate::policy::{run_driftwatch, DriftWatchConfig, PolicyOutcome};
use crate::registry::Registry;

use super::{
    redact_release_evidence, state_path_for, CapturedChangelog, CapturedCheck,
    ReleaseAdapterConfig, ReleaseConfig, ReleaseIdentity, ReleaseReport, ReleaseRequest,
    ReleaseState, StageOutcome, ADAPTER_TIMEOUT, CHECK_DISABLED, CHECK_FAIL, CHECK_PASS,
    CHECK_STALE, CHECK_UNAVAILABLE, RELEASE_CONTRACT_VERSION, STAGE_COMMIT, STAGE_CONTAINER,
    STAGE_DOCS, STAGE_MIRROR, STAGE_NOTES, STAGE_PACKAGE, STAGE_PUSH, STAGE_TAG, STATUS_CONFLICT,
    STATUS_DELIVERED, STATUS_DISABLED, STATUS_DIVERGED, STATUS_FAILED, STATUS_SKIPPED,
};

/// One configuration captured at prepare time. The apply step
/// reads this back from the persisted state to keep the
/// captured checks and the captured changelog bound to the
/// release identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedConfig {
    pub config: ReleaseConfig,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
}

/// Plan-only release report. The transport renders this
/// before any side effect runs; the contract refuses to apply
/// a plan that is not `ready`.
#[derive(Debug, Clone, Serialize)]
pub struct PlanReport {
    pub contract: String,
    pub project_id: String,
    pub identity: ReleaseIdentity,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
    pub checks: Vec<CapturedCheck>,
    pub stages: Vec<String>,
    pub note: String,
    pub ready: bool,
}

impl PlanReport {
    pub fn healthy(&self) -> bool {
        self.ready
    }
}

/// Render a [`PlanReport`] for human output.
pub fn render_plan_human(plan: &PlanReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", plan.project_id));
    lines.push(format!("release_id: {}", plan.identity.id));
    lines.push(format!("version: {}", plan.identity.version));
    lines.push(format!(
        "source_revision: {}",
        plan.identity.source_revision
    ));
    if let Some(changelog) = &plan.changelog {
        lines.push(format!("changelog: {}", changelog.path));
        lines.push(format!("changelog_hash: {}", changelog.content_hash));
    }
    if !plan.docs_locales.is_empty() {
        lines.push(format!("docs_locales: {}", plan.docs_locales.join(", ")));
    }
    lines.push(format!("stages: {}", plan.stages.join(", ")));
    lines.push(format!("ready: {}", if plan.ready { "yes" } else { "no" }));
    if !plan.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &plan.checks {
            lines.push(format!(
                "  - {kind} {status} applicable={applicable} revision={revision}: {detail}",
                kind = check.kind,
                status = check.status,
                applicable = check.applicable,
                revision = check.source_revision,
                detail = check.detail
            ));
        }
    }
    lines.push(format!("summary: {}", plan.note));
    lines.join("\n")
}

/// Build a read-only release plan. Captures the working-tree
/// revision, the changelog, the configured checks and the
/// available documentation locales. Runs the doctor, test and
/// DriftWatch checks and refuses to mark the plan `ready`
/// when any required check fails, is unavailable or is
/// stale. The apply step refuses to consume a plan whose
/// checks belong to a different revision.
pub fn prepare_release(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    request: &ReleaseRequest,
    policy: &DriftWatchConfig,
) -> Result<PlanReport, ForgeError> {
    request.validate()?;
    let revision = capture_source_revision(project_dir)?;
    let changelog = match load_release_changelog(project_dir, config) {
        Ok(c) => Some(c),
        Err(err) => {
            // A missing changelog blocks the plan: the
            // requirement brief binds changelog content to the
            // release.
            return Err(err);
        }
    };
    let docs_locales = effective_docs_locales(manifest, config);
    let checks = collect_captured_checks(project_dir, manifest, config, &revision, policy)?;
    let ready = checks
        .iter()
        .filter(|c| c.applicable)
        .all(|c| c.status == CHECK_PASS);
    let note = if ready {
        format!(
            "release plan ready for `{}`: {} check(s) passed at revision `{}`",
            request.project_id,
            checks.iter().filter(|c| c.status == CHECK_PASS).count(),
            short_revision(&revision)
        )
    } else {
        let failing: Vec<String> = checks
            .iter()
            .filter(|c| c.applicable && c.status != CHECK_PASS)
            .map(|c| format!("{}:{}", c.kind, c.status))
            .collect();
        format!(
            "release plan not ready: failing check(s) {}; release execution blocks before tag or publication",
            failing.join(", ")
        )
    };
    Ok(PlanReport {
        contract: RELEASE_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        identity: ReleaseIdentity::derive(&request.project_id, &request.version, &revision),
        changelog,
        docs_locales: docs_locales.clone(),
        checks,
        stages: effective_stages(config, request),
        note,
        ready,
    })
}

fn load_release_changelog(
    project_dir: &Path,
    config: &ReleaseConfig,
) -> Result<CapturedChangelog, ForgeError> {
    super::load_changelog(project_dir, &config.changelog)
}

/// Resolve the documentation locales that the release will
/// translate. When the manifest does not request any locale
/// the release omits the docs stage entirely, matching the
/// R1 boundary scenario. When the manifest lists a locale
/// that is disabled in the manifest's docs section, the
/// release refuses to translate it so a stage never runs
/// against a disabled locale.
fn effective_docs_locales(manifest: &Manifest, config: &ReleaseConfig) -> Vec<String> {
    let requested: Vec<String> = match &config.docs {
        Some(docs) => docs.translate.clone(),
        None => return Vec::new(),
    };
    let mut enabled: Vec<String> = Vec::new();
    if let Some(docs_meta) = &manifest.docs {
        for locale in requested {
            if let Some(entry) = docs_meta.translations.get(&locale) {
                if entry.enabled.unwrap_or(false) {
                    enabled.push(locale);
                }
            }
        }
    }
    enabled
}

fn effective_stages(config: &ReleaseConfig, request: &ReleaseRequest) -> Vec<String> {
    if !request.stages.is_empty() {
        return request.stages.clone();
    }
    config.stages.clone()
}

fn short_revision(rev: &str) -> &str {
    if rev.len() <= 12 {
        rev
    } else {
        &rev[..12]
    }
}

/// Run each configured check and produce a captured outcome.
/// The doctor check reuses the existing `run_doctor`
/// inventory; the test check reuses `run_test` from
/// `gitops`; the DriftWatch check reuses the policy adapter.
/// Unconfigured checks are reported as `disabled`.
fn collect_captured_checks(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    revision: &str,
    policy: &DriftWatchConfig,
) -> Result<Vec<CapturedCheck>, ForgeError> {
    let mut out: Vec<CapturedCheck> = Vec::new();
    let _ = manifest.project.profile.as_str();
    let configured: Vec<String> = config.checks.iter().map(|c| c.kind.clone()).collect();
    let known = ["doctor", "test", "driftwatch", "gate"];
    for kind in known {
        let applicable = configured.iter().any(|c| c == kind);
        let check = if !applicable {
            CapturedCheck {
                kind: kind.to_string(),
                status: CHECK_DISABLED.to_string(),
                applicable: false,
                evidence: vec![format!(
                    "check `{kind}` is not configured in release.checks"
                )],
                detail: format!("check `{kind}` is not configured; release does not require it"),
                source_revision: revision.to_string(),
            }
        } else {
            run_one_check(project_dir, manifest, kind, revision, policy)?
        };
        out.push(check);
    }
    Ok(out)
}

fn run_one_check(
    project_dir: &Path,
    manifest: &Manifest,
    kind: &str,
    revision: &str,
    policy: &DriftWatchConfig,
) -> Result<CapturedCheck, ForgeError> {
    match kind {
        "doctor" => Ok(run_doctor_check(project_dir, manifest, revision)),
        "test" => run_test_check(project_dir, manifest, revision),
        "driftwatch" => Ok(run_driftwatch_check(
            project_dir,
            manifest,
            revision,
            policy,
        )),
        "gate" => Ok(run_gate_check(project_dir, revision)),
        other => Err(ForgeError::ReleaseInvalid {
            reason: format!("release check kind `{other}` is not supported"),
        }),
    }
}

fn run_doctor_check(project_dir: &Path, manifest: &Manifest, revision: &str) -> CapturedCheck {
    let target = manifest
        .project
        .target_maturity
        .or(manifest.project.maturity)
        .unwrap_or(crate::core::manifest::Maturity::L1);
    let report = match crate::doctor::run_doctor(project_dir, Some(target), None, None) {
        Ok(r) => r,
        Err(err) => {
            return CapturedCheck {
                kind: "doctor".to_string(),
                status: CHECK_UNAVAILABLE.to_string(),
                applicable: true,
                evidence: vec![format!("doctor failed: {err}")],
                detail: "doctor is unavailable; release execution blocks".to_string(),
                source_revision: revision.to_string(),
            }
        }
    };
    if report.stale {
        return CapturedCheck {
            kind: "doctor".to_string(),
            status: CHECK_STALE.to_string(),
            applicable: true,
            evidence: vec!["doctor observation is stale; re-run forge register".to_string()],
            detail: "doctor is stale relative to the current forge.yaml".to_string(),
            source_revision: revision.to_string(),
        };
    }
    let blocking: Vec<String> = report
        .blocking_findings()
        .iter()
        .map(|f| f.id.clone())
        .collect();
    let unmet = report.unmet_controls().len();
    if !blocking.is_empty() || unmet > 0 {
        let mut evidence = blocking.clone();
        evidence.push(format!("{unmet} unmet maturity control(s)"));
        return CapturedCheck {
            kind: "doctor".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence,
            detail: format!("doctor reports {} blocking finding(s)", blocking.len()),
            source_revision: revision.to_string(),
        };
    }
    CapturedCheck {
        kind: "doctor".to_string(),
        status: CHECK_PASS.to_string(),
        applicable: true,
        evidence: vec!["doctor healthy at target maturity".to_string()],
        detail: "doctor is healthy at the configured target maturity".to_string(),
        source_revision: revision.to_string(),
    }
}

fn run_test_check(
    project_dir: &Path,
    manifest: &Manifest,
    revision: &str,
) -> Result<CapturedCheck, ForgeError> {
    let profile = match crate::profile::inspect_profile(&manifest.project.profile) {
        Ok(p) => p,
        Err(err) => {
            return Ok(CapturedCheck {
                kind: "test".to_string(),
                status: CHECK_UNAVAILABLE.to_string(),
                applicable: true,
                evidence: vec![format!("profile unavailable: {err}")],
                detail: "profile descriptor is unavailable; test cannot run".to_string(),
                source_revision: revision.to_string(),
            })
        }
    };
    let profile_id = manifest.project.profile.clone();
    let test_command = profile.test_command.clone();
    match run_test(
        &manifest.project.id,
        &profile_id,
        Some(&test_command),
        project_dir,
    ) {
        Ok(outcome) => {
            if outcome.status == "passed" {
                Ok(CapturedCheck {
                    kind: "test".to_string(),
                    status: CHECK_PASS.to_string(),
                    applicable: true,
                    evidence: vec![format!("test command `{}` passed", outcome.command)],
                    detail: "profile test command passed".to_string(),
                    source_revision: revision.to_string(),
                })
            } else {
                Ok(CapturedCheck {
                    kind: "test".to_string(),
                    status: CHECK_FAIL.to_string(),
                    applicable: true,
                    evidence: outcome.evidence,
                    detail: "test command did not pass".to_string(),
                    source_revision: revision.to_string(),
                })
            }
        }
        Err(ForgeError::TestFailed { reason }) => Ok(CapturedCheck {
            kind: "test".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence: vec![reason],
            detail: "test command did not pass".to_string(),
            source_revision: revision.to_string(),
        }),
        Err(err) => Ok(CapturedCheck {
            kind: "test".to_string(),
            status: CHECK_UNAVAILABLE.to_string(),
            applicable: true,
            evidence: vec![err.to_string()],
            detail: "test command could not start".to_string(),
            source_revision: revision.to_string(),
        }),
    }
}

fn run_driftwatch_check(
    project_dir: &Path,
    manifest: &Manifest,
    revision: &str,
    policy: &DriftWatchConfig,
) -> CapturedCheck {
    let outcome = run_driftwatch(project_dir, policy);
    let _ = manifest;
    match outcome {
        PolicyOutcome::Reported(report) => {
            let mut fail_ids: Vec<String> = Vec::new();
            for finding in &report.findings {
                if !finding.applicable {
                    continue;
                }
                if matches!(finding.severity, crate::policy::PolicySeverity::Fail) {
                    fail_ids.push(finding.id.clone());
                }
            }
            if !fail_ids.is_empty() {
                CapturedCheck {
                    kind: "driftwatch".to_string(),
                    status: CHECK_FAIL.to_string(),
                    applicable: true,
                    evidence: fail_ids,
                    detail: "driftwatch reports failing rules".to_string(),
                    source_revision: revision.to_string(),
                }
            } else {
                CapturedCheck {
                    kind: "driftwatch".to_string(),
                    status: CHECK_PASS.to_string(),
                    applicable: true,
                    evidence: vec!["driftwatch report healthy".to_string()],
                    detail: "driftwatch report is healthy".to_string(),
                    source_revision: revision.to_string(),
                }
            }
        }
        PolicyOutcome::Unavailable { reason } => CapturedCheck {
            kind: "driftwatch".to_string(),
            status: CHECK_UNAVAILABLE.to_string(),
            applicable: true,
            evidence: vec![reason],
            detail: "driftwatch adapter is unavailable".to_string(),
            source_revision: revision.to_string(),
        },
    }
}

/// Release `gate` check. This never executes the runtime — `forge gate`
/// owns execution — it only cites the persisted, revision-bound evidence
/// record: pass on a fresh passing aggregate, `stale` when the evidence
/// is bound to another (or no) revision, fail on a blocked, failing or
/// unclassifiable aggregate, and `unavailable` when the gate has never
/// run or the record cannot be read. Absent evidence never passes.
fn run_gate_check(project_dir: &Path, revision: &str) -> CapturedCheck {
    let evidence = match crate::gate::load_latest_evidence(project_dir) {
        Ok(Some(evidence)) => evidence,
        Ok(None) => {
            return CapturedCheck {
                kind: "gate".to_string(),
                status: CHECK_UNAVAILABLE.to_string(),
                applicable: true,
                evidence: vec![format!("{}/", crate::gate::GATE_EVIDENCE_DIR)],
                detail: "the gate has never run for this project; unverified evidence cannot back a release".to_string(),
                source_revision: revision.to_string(),
            };
        }
        Err(err) => {
            return CapturedCheck {
                kind: "gate".to_string(),
                status: CHECK_UNAVAILABLE.to_string(),
                applicable: true,
                evidence: vec![err.to_string()],
                detail: "persisted gate evidence cannot be read; release execution blocks"
                    .to_string(),
                source_revision: revision.to_string(),
            };
        }
    };
    let summary = crate::gate::evidence_summary(&evidence);
    let freshness = crate::gate::evidence_freshness(&evidence, Some(revision));
    if freshness == crate::gate::GateFreshness::Stale {
        return CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_STALE.to_string(),
            applicable: true,
            evidence: vec![summary],
            detail: "gate evidence is bound to another (or no) revision; stale evidence cannot back a release claim".to_string(),
            source_revision: revision.to_string(),
        };
    }
    match evidence.aggregate {
        crate::gate::GateAggregate::Passed => CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_PASS.to_string(),
            applicable: true,
            evidence: vec![summary],
            detail: "the shared gate runtime passed at this revision".to_string(),
            source_revision: revision.to_string(),
        },
        crate::gate::GateAggregate::Blocked => CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence: vec![summary],
            detail: "the gate runtime blocked this revision".to_string(),
            source_revision: revision.to_string(),
        },
        crate::gate::GateAggregate::Failed => CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence: vec![summary],
            detail: "the gate runtime reported a failure on this revision".to_string(),
            source_revision: revision.to_string(),
        },
        crate::gate::GateAggregate::Unknown => CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence: vec![summary],
            detail: "the gate aggregate is unclassifiable and cannot back a release claim"
                .to_string(),
            source_revision: revision.to_string(),
        },
    }
}

/// Apply a verified plan. The plan must be `ready` and the
/// captured source revision must match the working-tree
/// revision; otherwise the apply step refuses with
/// `release-check-failed` and a recovery note. Each stage is
/// recorded with a stable status so a retry resumes from the
/// last delivered stage.
pub fn apply_release(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    request: &ReleaseRequest,
    adapters: &ReleaseAdapterConfig,
) -> Result<ReleaseReport, ForgeError> {
    request.validate()?;
    let revision = capture_source_revision(project_dir)?;
    let identity = ReleaseIdentity::derive(&request.project_id, &request.version, &revision);
    let state_path = state_path_for(project_dir, &request.project_id, &identity)?;
    let mut state = super::load_release_state(&state_path)?;
    let changelog = load_release_changelog(project_dir, config)?;
    let docs_locales = effective_docs_locales(manifest, config);
    let stages = effective_stages(config, request);

    if state.contract.is_empty() {
        state.contract = RELEASE_CONTRACT_VERSION.to_string();
        state.identity = identity.clone();
        state.changelog = Some(changelog.clone());
        state.docs_locales = docs_locales.clone();
        state.stages = stages.clone();
    } else {
        // Identity bound: the persisted state belongs to the
        // exact project+version+revision triple. Anything else
        // is a different release attempt.
        if state.identity != identity {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "release state at `{}` belongs to release id `{}`, not `{}`; rerun with the matching version or revision",
                    state_path.display(),
                    state.identity.id,
                    identity.id
                ),
            });
        }
        if !stages.is_empty() {
            state.stages = stages.clone();
        }
    }

    // Re-run the captured checks so a stale plan cannot be
    // applied after the working tree has moved on.
    let policy = DriftWatchConfig::from_env();
    let checks = collect_captured_checks(project_dir, manifest, config, &revision, &policy)?;
    let ready = checks
        .iter()
        .filter(|c| c.applicable)
        .all(|c| c.status == CHECK_PASS);
    if !ready {
        let failing: Vec<String> = checks
            .iter()
            .filter(|c| c.applicable && c.status != CHECK_PASS)
            .map(|c| format!("{}:{}", c.kind, c.status))
            .collect();
        return Err(ForgeError::ReleaseCheckFailed {
            reason: format!(
                "release check(s) {} at revision `{}` are not passing; release execution blocks before tag or publication",
                failing.join(", "),
                short_revision(&revision)
            ),
        });
    }
    if !request.confirm && !request.dry_run {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release apply requires --confirm; refusing implicit side effects".to_string(),
        });
    }
    state.checks = checks.clone();

    let mut stage_outcomes: Vec<StageOutcome> = Vec::new();
    let mut deliverable_count = 0usize;
    for stage in &stages {
        let outcome = if request.retry && already_delivered(&state, stage, &revision) {
            StageOutcome {
                stage: stage.to_string(),
                target: stage_target(stage, manifest, config),
                status: STATUS_SKIPPED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "stage `{stage}` already delivered at revision `{}`; retry skips",
                    short_revision(&revision)
                ),
                evidence: Vec::new(),
                recovery: Vec::new(),
            }
        } else {
            let outcome = run_stage(
                project_dir,
                manifest,
                config,
                stage,
                &identity,
                &revision,
                adapters,
                request,
            )?;
            if outcome.status == STATUS_DELIVERED {
                deliverable_count += 1;
            }
            outcome
        };
        state.stage_outcomes.push(outcome.clone());
        stage_outcomes.push(outcome);
    }
    state.last_run_at = chrono::Utc::now().to_rfc3339();
    super::save_release_state(&state_path, &state)?;

    let dry_run = request.dry_run;
    let retry = request.retry;
    let healthy = stage_outcomes.iter().all(|o| {
        matches!(
            o.status.as_str(),
            STATUS_DELIVERED | STATUS_SKIPPED | STATUS_DISABLED
        )
    }) && !stage_outcomes.is_empty();
    let note = if dry_run {
        "dry-run: no side effects were attempted".to_string()
    } else if healthy {
        format!(
            "release `{}` delivered; {deliverable_count} stage(s) recorded for revision `{}`",
            identity.id,
            short_revision(&revision)
        )
    } else {
        let failing: Vec<String> = stage_outcomes
            .iter()
            .filter(|o| {
                !matches!(
                    o.status.as_str(),
                    STATUS_DELIVERED | STATUS_SKIPPED | STATUS_DISABLED
                )
            })
            .map(|o| format!("{}:{}", o.stage, o.status))
            .collect();
        format!(
            "release `{}` is partial; failing stage(s) {}; prior stage state is preserved",
            identity.id,
            failing.join(", ")
        )
    };
    Ok(ReleaseReport {
        contract: RELEASE_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        identity,
        changelog: Some(changelog),
        docs_locales,
        checks,
        stages,
        stage_outcomes,
        dry_run,
        retry,
        state_path: state_path.display().to_string(),
        note,
        healthy,
    })
}

fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    super::capture_source_revision(dir)
}

fn already_delivered(state: &ReleaseState, stage: &str, revision: &str) -> bool {
    state
        .stage_outcomes
        .iter()
        .any(|o| o.stage == stage && (o.status == STATUS_DELIVERED || o.status == STATUS_SKIPPED))
        && state.identity.source_revision == revision
}

fn stage_target(stage: &str, manifest: &Manifest, config: &ReleaseConfig) -> String {
    match stage {
        STAGE_COMMIT => "working-tree".to_string(),
        STAGE_TAG => format!("refs/tags/{}", manifest.project.id),
        STAGE_PUSH => "origin".to_string(),
        STAGE_MIRROR => "distribution/mirrors".to_string(),
        STAGE_PACKAGE => config
            .packages
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        STAGE_CONTAINER => config
            .containers
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        STAGE_DOCS => "docs/translate".to_string(),
        STAGE_NOTES => config
            .notes
            .as_ref()
            .map(|n| n.output.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        _ => "(unknown)".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    stage: &str,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    match stage {
        STAGE_COMMIT => run_commit_stage(project_dir, manifest, config, identity, revision),
        STAGE_TAG => run_tag_stage(project_dir, manifest, identity, revision, request),
        STAGE_PUSH => run_push_stage(project_dir, manifest, identity, revision, request),
        STAGE_MIRROR => run_mirror_stage(project_dir, manifest, identity, revision, request),
        STAGE_PACKAGE => {
            run_package_stage(project_dir, config, identity, revision, adapters, request)
        }
        STAGE_CONTAINER => {
            run_container_stage(project_dir, config, identity, revision, adapters, request)
        }
        STAGE_DOCS => run_docs_stage(project_dir, manifest, config, identity, revision),
        STAGE_NOTES => run_notes_stage(project_dir, config, identity, revision, adapters, request),
        other => Ok(StageOutcome {
            stage: other.to_string(),
            target: "(unknown)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("stage `{other}` is not recognized; treating as disabled"),
            evidence: Vec::new(),
            recovery: Vec::new(),
        }),
    }
}

fn run_commit_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
) -> Result<StageOutcome, ForgeError> {
    // The release commit bundles the changelog (and any
    // release-managed files) into one commit. The user can
    // override the path list with `release.commit.paths` is
    // out of scope; the default is the changelog only so
    // unrelated working-tree edits are never silently pulled
    // in.
    let path = config.changelog.clone();
    if !project_dir.join(&path).is_file() {
        return Ok(StageOutcome {
            stage: STAGE_COMMIT.to_string(),
            target: path.clone(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!(
                "no changelog to commit at `{}`; stage `{}` skipped",
                path, STAGE_COMMIT
            ),
            evidence: Vec::new(),
            recovery: Vec::new(),
        });
    }
    let message = format!(
        "release {} ({})",
        identity.version,
        short_revision(revision)
    );
    let outcome = match commit_paths(
        &manifest.project.id,
        project_dir,
        std::slice::from_ref(&path),
        &message,
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(StageOutcome {
                stage: STAGE_COMMIT.to_string(),
                target: path,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("commit stage failed: {err}"),
                evidence: vec![err.to_string()],
                recovery: vec![
                    "stage the changelog explicitly and re-run forge release apply".to_string(),
                ],
            });
        }
    };
    Ok(StageOutcome {
        stage: STAGE_COMMIT.to_string(),
        target: path,
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!(
            "commit stage delivered release `{}` for project `{}`",
            identity.id, manifest.project.id
        ),
        evidence: vec![format!(
            "files: {}",
            if outcome.files_changed.is_empty() {
                "(none)".to_string()
            } else {
                outcome.files_changed.join(", ")
            }
        )],
        recovery: Vec::new(),
    })
}

fn run_tag_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    _request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let tag = format!("v{}", identity.version);
    // The tag is annotated so the release identity is
    // preserved alongside the commit. A pre-existing tag at a
    // different commit is a `release-identity-conflict`; a
    // pre-existing tag at the same commit is `skipped` so
    // retries are idempotent.
    let existing = read_tag_target(project_dir, &tag)?;
    if let Some(existing_sha) = existing {
        if existing_sha != revision {
            return Ok(StageOutcome {
                stage: STAGE_TAG.to_string(),
                target: tag.clone(),
                status: STATUS_CONFLICT.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "tag `{tag}` already exists at commit `{existing_sha}`; refusing to replace it with `{revision}`"
                ),
                evidence: vec![format!("existing: {existing_sha}"), format!("requested: {revision}")],
                recovery: vec![
                    "remove the conflicting tag with `git tag -d {tag}` and re-run forge release apply".to_string(),
                    "or release at a different semver to keep history immutable".to_string(),
                ],
            });
        }
        return Ok(StageOutcome {
            stage: STAGE_TAG.to_string(),
            target: tag.clone(),
            status: STATUS_SKIPPED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("tag `{tag}` already points at `{revision}`; tag stage skipped"),
            evidence: Vec::new(),
            recovery: Vec::new(),
        });
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("tag")
        .arg("-a")
        .arg(&tag)
        .arg("-m")
        .arg(format!(
            "release {} for {}",
            identity.version, manifest.project.id
        ))
        .arg(revision)
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git tag invocation failed: {err}"),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(StageOutcome {
            stage: STAGE_TAG.to_string(),
            target: tag.clone(),
            status: STATUS_FAILED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("git tag -a `{tag}` failed"),
            evidence: vec![redact_release_evidence(&stderr)],
            recovery: vec![format!(
                "verify the working tree at `{revision}` is reachable"
            )],
        });
    }
    Ok(StageOutcome {
        stage: STAGE_TAG.to_string(),
        target: tag.clone(),
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!("tag `{tag}` delivered for release `{}`", identity.id),
        evidence: vec![format!("annotated tag at {revision}")],
        recovery: Vec::new(),
    })
}

fn read_tag_target(project_dir: &Path, tag: &str) -> Result<Option<String>, ForgeError> {
    // Peel annotated tags to the underlying commit so the
    // comparison against the working-tree revision is the
    // commit SHA, not the tag object SHA.
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("rev-parse")
        .arg("--verify")
        .arg(format!("refs/tags/{tag}^{{commit}}"))
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !output.status.success() {
        // Lightweight tags do not peel; fall back to the
        // raw SHA so the comparison still works.
        let output = Command::new("git")
            .arg("-C")
            .arg(project_dir)
            .arg("rev-parse")
            .arg("--verify")
            .arg(format!("refs/tags/{tag}"))
            .output()
            .map_err(|err| ForgeError::ReleaseInvalid {
                reason: format!("git rev-parse failed: {err}"),
            })?;
        if !output.status.success() {
            return Ok(None);
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if sha.is_empty() {
            return Ok(None);
        }
        return Ok(Some(sha));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sha))
    }
}

fn run_push_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let tag = format!("v{}", identity.version);
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_PUSH.to_string(),
            target: format!("origin:{tag}"),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "push stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to push the release tag to the primary remote".to_string(),
            ],
        });
    }
    let outcome = match push_ref(&manifest.project.id, project_dir, "origin", &tag, true) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(StageOutcome {
                stage: STAGE_PUSH.to_string(),
                target: format!("origin:{tag}"),
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("push stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err.to_string())],
                recovery: vec![
                    "verify the `origin` remote and the credential helper, then re-run forge release apply".to_string()
                ],
            });
        }
    };
    let _ = revision;
    Ok(StageOutcome {
        stage: STAGE_PUSH.to_string(),
        target: format!("origin:{tag}"),
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!(
            "push stage delivered `{tag}` to `origin` for release `{}`",
            identity.id
        ),
        evidence: outcome
            .commit_sha
            .map(|s| format!("commit: {s}"))
            .into_iter()
            .collect(),
        recovery: Vec::new(),
    })
}

fn run_mirror_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let config: DistributionConfig = match manifest.distribution.as_ref() {
        Some(_) => match distribution_config_from_manifest(manifest) {
            Ok(c) => c,
            Err(err) => {
                return Ok(StageOutcome {
                    stage: STAGE_MIRROR.to_string(),
                    target: "distribution/mirrors".to_string(),
                    status: STATUS_FAILED.to_string(),
                    identity: Some(identity.id.clone()),
                    note: format!("mirror stage failed: {err}"),
                    evidence: vec![err.to_string()],
                    recovery: vec![
                        "declare a `distribution` section in the manifest with a primary or at least one mirror".to_string(),
                    ],
                });
            }
        },
        None => {
            return Ok(StageOutcome {
                stage: STAGE_MIRROR.to_string(),
                target: "distribution/mirrors".to_string(),
                status: STATUS_DISABLED.to_string(),
                identity: Some(identity.id.clone()),
                note: "no `distribution` section in the manifest; mirror stage skipped".to_string(),
                evidence: Vec::new(),
                recovery: vec![
                    "add a `distribution` section to the manifest to publish mirrors as part of the release".to_string(),
                ],
            });
        }
    };
    let tag = format!("v{}", identity.version);
    let mirror_request = MirrorRequest {
        project_id: manifest.project.id.clone(),
        refs: vec![tag.clone()],
        confirm: request.confirm,
        dry_run: request.dry_run,
        retry_failed: request.retry,
    };
    if !request.confirm {
        // Plan-only path: surface what would happen.
        let state_path = crate::distribution::state_path_for(project_dir, &manifest.project.id)
            .unwrap_or_else(|_| project_dir.join(".forge/distribution/state.json"));
        let state = crate::distribution::load_mirror_state(&state_path).unwrap_or_default();
        let plan = plan_distribution(&config, &mirror_request, &state, &state_path)?;
        let primary = plan
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .map(|o| o.status.clone())
            .unwrap_or_else(|| "absent".to_string());
        let mirror_count = plan
            .outcomes
            .iter()
            .filter(|o| o.role == "mirror" && o.status == "would-push")
            .count();
        return Ok(StageOutcome {
            stage: STAGE_MIRROR.to_string(),
            target: "distribution/mirrors".to_string(),
            status: if plan.healthy() {
                STATUS_DELIVERED.to_string()
            } else {
                STATUS_FAILED.to_string()
            },
            identity: Some(identity.id.clone()),
            note: format!(
                "mirror plan only: primary `{primary}`, {mirror_count} mirror(s) scheduled for `{tag}` (retry)"
            ),
            evidence: plan
                .outcomes
                .iter()
                .map(|o| format!("{} {} {}", o.role, o.provider, o.status))
                .collect(),
            recovery: Vec::new(),
        });
    }
    let _ = revision;
    let report = apply_mirror(project_dir, &config, &mirror_request)?;
    let primary = report
        .outcomes
        .iter()
        .find(|o| o.role == "primary")
        .map(|o| o.status.clone())
        .unwrap_or_else(|| "absent".to_string());
    let delivered_mirrors = report
        .outcomes
        .iter()
        .filter(|o| o.role == "mirror" && o.status == STATUS_DELIVERED)
        .count();
    let overall = if report.healthy() {
        STATUS_DELIVERED.to_string()
    } else if primary == STATUS_DIVERGED {
        STATUS_DIVERGED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    Ok(StageOutcome {
        stage: STAGE_MIRROR.to_string(),
        target: "distribution/mirrors".to_string(),
        status: overall,
        identity: Some(identity.id.clone()),
        note: format!(
            "mirror stage: primary `{primary}`, {delivered_mirrors} mirror(s) delivered for `{tag}`"
        ),
        evidence: report
            .outcomes
            .iter()
            .map(|o| format!("{} {} {}", o.role, o.provider, o.status))
            .collect(),
        recovery: report
            .outcomes
            .iter()
            .filter(|o| o.status == STATUS_FAILED || o.status == STATUS_DIVERGED)
            .flat_map(|o| o.recovery.clone())
            .collect(),
    })
}

fn run_package_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    if config.packages.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_PACKAGE.to_string(),
            target: "(none)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no packages configured in `release.packages`; package stage skipped".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "declare `release.packages` entries to publish one package per release".to_string(),
            ],
        });
    }
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_PACKAGE.to_string(),
            target: config
                .packages
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "(none)".to_string()),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "package stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to publish packages through the package adapter".to_string(),
            ],
        });
    }
    let _ = revision;
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for package in &config.packages {
        let target = format!("{}@{}", package.name, package.version);
        match run_package_adapter(&adapters.package_bin, project_dir, package, identity) {
            Ok(receipt) => outcomes.push(StageOutcome {
                stage: STAGE_PACKAGE.to_string(),
                target: target.clone(),
                status: STATUS_DELIVERED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("package `{target}` delivered for release `{}`", identity.id),
                evidence: vec![format!("receipt: {receipt}")],
                recovery: Vec::new(),
            }),
            Err(err) => outcomes.push(StageOutcome {
                stage: STAGE_PACKAGE.to_string(),
                target,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("package stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err)],
                recovery: vec![format!(
                    "verify the package adapter `{}` and re-run forge release apply",
                    adapters.package_bin
                )],
            }),
        }
    }
    // Aggregate to one StageOutcome so the per-stage record
    // is stable; the per-package evidence is preserved in
    // `evidence` lines.
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let note = if all_delivered {
        format!(
            "{} package(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} package(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    Ok(StageOutcome {
        stage: STAGE_PACKAGE.to_string(),
        target: "packages".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

fn run_container_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    if config.containers.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_CONTAINER.to_string(),
            target: "(none)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no containers configured in `release.containers`; container stage skipped"
                .to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "declare `release.containers` entries to publish one image per release".to_string(),
            ],
        });
    }
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_CONTAINER.to_string(),
            target: config
                .containers
                .first()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "(none)".to_string()),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "container stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to publish container images through the container adapter"
                    .to_string(),
            ],
        });
    }
    let _ = revision;
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for container in &config.containers {
        let target = format!("{}:{}", container.name, container.tag);
        match run_container_adapter(&adapters.container_bin, project_dir, container, identity) {
            Ok(receipt) => outcomes.push(StageOutcome {
                stage: STAGE_CONTAINER.to_string(),
                target: target.clone(),
                status: STATUS_DELIVERED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "container `{target}` delivered for release `{}`",
                    identity.id
                ),
                evidence: vec![format!("receipt: {receipt}")],
                recovery: Vec::new(),
            }),
            Err(err) => outcomes.push(StageOutcome {
                stage: STAGE_CONTAINER.to_string(),
                target,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("container stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err)],
                recovery: vec![format!(
                    "verify the container adapter `{}` and re-run forge release apply",
                    adapters.container_bin
                )],
            }),
        }
    }
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    let note = if all_delivered {
        format!(
            "{} container(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} container(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    Ok(StageOutcome {
        stage: STAGE_CONTAINER.to_string(),
        target: "containers".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

fn run_docs_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
) -> Result<StageOutcome, ForgeError> {
    let locales: Vec<String> = effective_docs_locales(manifest, config);
    if locales.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_DOCS.to_string(),
            target: "docs/translate".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no enabled translation locale configured; docs stage skipped".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "enable a locale in `docs.translations.<locale>.enabled` and add it to `release.docs.translate` to translate during the release".to_string(),
            ],
        });
    }
    let _ = revision;
    let translator = crate::docs::TranslatorConfig::from_env();
    let translator_default = translator.binary.to_string_lossy().to_string();
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for locale in &locales {
        let request = crate::docs::TranslateRequest {
            project_id: manifest.project.id.clone(),
            locale: Some(locale.clone()),
            all: false,
        };
        let docs_config = match crate::docs::docs_config_from_manifest(manifest) {
            Ok(c) => c,
            Err(err) => {
                outcomes.push(StageOutcome {
                    stage: STAGE_DOCS.to_string(),
                    target: format!("docs/translate:{locale}"),
                    status: STATUS_FAILED.to_string(),
                    identity: Some(identity.id.clone()),
                    note: format!("docs stage failed to read config: {err}"),
                    evidence: vec![err.to_string()],
                    recovery: vec!["verify the `docs` section in the manifest".to_string()],
                });
                continue;
            }
        };
        let report =
            match crate::docs::run_translate(project_dir, &docs_config, &request, &translator) {
                Ok(report) => report,
                Err(err) => {
                    outcomes.push(StageOutcome {
                        stage: STAGE_DOCS.to_string(),
                        target: format!("docs/translate:{locale}"),
                        status: STATUS_FAILED.to_string(),
                        identity: Some(identity.id.clone()),
                        note: format!("docs stage failed: {err}"),
                        evidence: vec![redact_release_evidence(&err.to_string())],
                        recovery: vec![format!(
                            "verify the translator `{}` and re-run forge release apply",
                            translator_default
                        )],
                    });
                    continue;
                }
            };
        let outcome = report
            .outcomes
            .iter()
            .find(|o| o.locale == *locale)
            .cloned()
            .unwrap_or(crate::docs::TranslateOutcome {
                locale: locale.clone(),
                status: crate::docs::STATUS_FAILED.to_string(),
                source_hash: None,
                segments_translated: 0,
                segments_reused: 0,
                derivative: None,
                review: "failed".to_string(),
                review_reasons: Vec::new(),
                note: "locale was not present in the docs report".to_string(),
                evidence: Vec::new(),
                recovery: Vec::new(),
            });
        let status = if outcome.status == crate::docs::STATUS_TRANSLATED
            || outcome.status == crate::docs::STATUS_CURRENT
        {
            STATUS_DELIVERED.to_string()
        } else {
            STATUS_FAILED.to_string()
        };
        outcomes.push(StageOutcome {
            stage: STAGE_DOCS.to_string(),
            target: format!("docs/translate:{locale}"),
            status,
            identity: Some(identity.id.clone()),
            note: format!("docs stage for locale `{locale}`: {}", outcome.note),
            evidence: outcome.evidence,
            recovery: outcome.recovery,
        });
    }
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    let note = if all_delivered {
        format!(
            "{} docs locale(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} docs locale(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    Ok(StageOutcome {
        stage: STAGE_DOCS.to_string(),
        target: "docs/translate".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

fn run_notes_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let notes = match &config.notes {
        Some(notes) => notes,
        None => {
            return Ok(StageOutcome {
                stage: STAGE_NOTES.to_string(),
                target: "(none)".to_string(),
                status: STATUS_DISABLED.to_string(),
                identity: Some(identity.id.clone()),
                note: "no `release.notes` configured; notes stage skipped".to_string(),
                evidence: Vec::new(),
                recovery: vec![
                    "declare a `release.notes.template` and `release.notes.output` to render notes during the release".to_string(),
                ],
            });
        }
    };
    let _ = (revision, request);
    match run_notes_adapter(&adapters.notes_bin, project_dir, notes, identity, request) {
        Ok(receipt) => Ok(StageOutcome {
            stage: STAGE_NOTES.to_string(),
            target: notes.output.clone(),
            status: STATUS_DELIVERED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!(
                "notes rendered to `{}` for release `{}`",
                notes.output, identity.id
            ),
            evidence: vec![format!("receipt: {receipt}")],
            recovery: Vec::new(),
        }),
        Err(err) => Ok(StageOutcome {
            stage: STAGE_NOTES.to_string(),
            target: notes.output.clone(),
            status: STATUS_FAILED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("notes stage failed: {err}"),
            evidence: vec![redact_release_evidence(&err)],
            recovery: vec![format!(
                "verify the notes adapter `{}` and re-run forge release apply",
                adapters.notes_bin
            )],
        }),
    }
}

fn run_package_adapter(
    bin: &str,
    project_dir: &Path,
    package: &super::PackageSpec,
    identity: &ReleaseIdentity,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--package-kind")
        .arg(&package.kind)
        .arg("--package-name")
        .arg(&package.name)
        .arg("--package-path")
        .arg(&package.path)
        .arg("--package-version")
        .arg(&package.version)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output =
        run_with_timeout(cmd, ADAPTER_TIMEOUT, "package adapter").map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "package adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!("package adapter `{bin}` returned an empty receipt"));
    }
    Ok(receipt)
}

fn run_container_adapter(
    bin: &str,
    project_dir: &Path,
    container: &super::ContainerSpec,
    identity: &ReleaseIdentity,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--container-name")
        .arg(&container.name)
        .arg("--dockerfile")
        .arg(&container.dockerfile)
        .arg("--container-tag")
        .arg(&container.tag)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir);
    if let Some(registry) = &container.registry {
        cmd.arg("--registry").arg(registry);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = run_with_timeout(cmd, ADAPTER_TIMEOUT, "container adapter")
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "container adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!(
            "container adapter `{bin}` returned an empty receipt"
        ));
    }
    Ok(receipt)
}

fn run_notes_adapter(
    bin: &str,
    project_dir: &Path,
    notes: &super::NotesSpec,
    identity: &ReleaseIdentity,
    request: &ReleaseRequest,
) -> Result<String, String> {
    let mut cmd = Command::new(bin);
    cmd.arg("--contract")
        .arg(RELEASE_CONTRACT_VERSION)
        .arg("--template")
        .arg(&notes.template)
        .arg("--output")
        .arg(&notes.output)
        .arg("--release-id")
        .arg(&identity.id)
        .arg("--source-revision")
        .arg(&identity.source_revision)
        .arg("--project-dir")
        .arg(project_dir)
        .arg("--dry-run")
        .arg(if request.dry_run { "true" } else { "false" });
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output =
        run_with_timeout(cmd, ADAPTER_TIMEOUT, "notes adapter").map_err(|err| err.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "notes adapter `{bin}` exited {}: {} {}",
            output.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt = stdout.lines().next().unwrap_or("").trim().to_string();
    if receipt.is_empty() {
        return Err(format!("notes adapter `{bin}` returned an empty receipt"));
    }
    Ok(receipt)
}

fn run_with_timeout(
    mut cmd: Command,
    timeout: std::time::Duration,
    label: &str,
) -> Result<std::process::Output, String> {
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("{label} spawn failed: {err}"))?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return wait_with_output(child, status),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{label} exceeded the {timeout:?} timeout"));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{label} wait failed: {err}"));
            }
        }
    }
}

fn wait_with_output(
    mut child: std::process::Child,
    status: std::process::ExitStatus,
) -> Result<std::process::Output, String> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_thread = stdout.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
            buf
        })
    });
    let stderr_thread = stderr.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
            buf
        })
    });
    let _ = child.wait();
    let stdout = stdout_thread
        .and_then(|t| t.join().ok())
        .unwrap_or_default();
    let stderr = stderr_thread
        .and_then(|t| t.join().ok())
        .unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

/// List every persisted release under the project. The
/// result is the inventory the CLI renders for
/// `forge release list`.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseListEntry {
    pub project_id: String,
    pub release_id: String,
    pub version: String,
    pub source_revision: String,
    pub stage_count: usize,
    pub last_run_at: String,
}

pub fn list_releases(
    project_dir: &Path,
    project_id: &str,
) -> Result<Vec<ReleaseListEntry>, ForgeError> {
    let root = project_dir.join(super::RELEASE_STATE_DIR).join(project_id);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<ReleaseListEntry> = Vec::new();
    for dir in fs::read_dir(&root).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read release directory {}: {err}", root.display()),
    })? {
        let entry = match dir {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path().join("state.json");
        let state = match super::load_release_state(&path) {
            Ok(state) => state,
            Err(_) => continue,
        };
        entries.push(ReleaseListEntry {
            project_id: state.identity.project_id.clone(),
            release_id: state.identity.id.clone(),
            version: state.identity.version.clone(),
            source_revision: state.identity.source_revision.clone(),
            stage_count: state.stage_outcomes.len(),
            last_run_at: state.last_run_at.clone(),
        });
    }
    entries.sort_by(|a, b| b.last_run_at.cmp(&a.last_run_at));
    Ok(entries)
}

/// Inspect one persisted release. Returns `Ok(None)` when the
/// release id is not found.
pub fn read_release(
    project_dir: &Path,
    project_id: &str,
    release_id: &str,
) -> Result<Option<ReleaseState>, ForgeError> {
    if release_id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release id must not be empty".to_string(),
        });
    }
    let path = project_dir
        .join(super::RELEASE_STATE_DIR)
        .join(project_id)
        .join(release_id)
        .join("state.json");
    if !path.is_file() {
        return Ok(None);
    }
    let state = super::load_release_state(&path)?;
    Ok(Some(state))
}

/// Journal a release operation in the registry so the
/// originating intent is preserved. The CLI and the MCP
/// transport call this after every prepare/apply; the kind
/// is `release`.
pub fn record_release_operation(
    registry: &Registry,
    project_id: &str,
    state_label: &str,
    detail: &str,
) {
    let _ = registry.record_operation("release", project_id, state_label, detail);
}

#[cfg(test)]
mod tests {
    use super::super::{release_config_from_manifest, Semver};
    use super::*;
    use crate::core::manifest::Maturity;
    use crate::policy::DriftWatchConfig;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(cwd: &Path, args: &[&str]) {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd);
        for a in args {
            cmd.arg(a);
        }
        let out = cmd.output().expect("git invocation");
        assert!(
            out.status.success(),
            "git {:?} failed: stderr={}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write_release_manifest(dir: &Path, id: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        let text = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n  changelog: CHANGELOG.md\n{body}"
        );
        fs::write(dir.join("forge.yaml"), text).unwrap();
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
        // Provide a minimal but valid Cargo workspace so
        // the doctor build-config check and the `cargo
        // test` command both pass without contacting the
        // network. The tests are the only thing the
        // contract claims about the test command; an empty
        // test set reports as a passing test run.
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"forge-rel-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
        )
        .unwrap();
        fs::write(dir.join("lib.rs"), "//! Forge release fixture crate.\n").unwrap();
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "forge@example.com"]);
        git(dir, &["config", "user.name", "Forge Test"]);
        git(dir, &["config", "init.defaultBranch", "main"]);
        git(
            dir,
            &[
                "add",
                "--",
                "forge.yaml",
                "README.md",
                "CHANGELOG.md",
                "Cargo.toml",
                "lib.rs",
            ],
        );
        git(dir, &["commit", "-q", "-m", "initial"]);
    }

    fn read_manifest(dir: &Path) -> Manifest {
        let (manifest, _) = Manifest::load_from_dir(dir, None).expect("manifest");
        manifest
    }

    fn read_release_config(dir: &Path) -> ReleaseConfig {
        let manifest = read_manifest(dir);
        release_config_from_manifest(&manifest).expect("release config")
    }

    #[test]
    fn prepare_captures_changelog_and_disabled_checks() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-prep", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.2.3").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        assert!(plan.changelog.is_some());
        // doctor/test checks are not configured in the
        // fixture's `release.checks` (it only lists the
        // kinds the contract claims to support); the
        // boundary scenario is exercised below.
        let kinds: Vec<&str> = plan.checks.iter().map(|c| c.kind.as_str()).collect();
        assert!(kinds.contains(&"doctor"));
        assert!(kinds.contains(&"test"));
        assert!(kinds.contains(&"driftwatch"));
    }

    #[test]
    fn prepare_marks_check_unavailable_when_doctor_fails() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        fs::create_dir_all(dir).unwrap();
        // Empty working tree (no Cargo.toml, no
        // forge.yaml); init a git repo with a commit so
        // the source revision capture can succeed.
        git(dir, &["init", "-q"]);
        git(dir, &["config", "user.email", "forge@example.com"]);
        git(dir, &["config", "user.name", "Forge Test"]);
        git(dir, &["config", "init.defaultBranch", "main"]);
        git(dir, &["checkout", "-q", "-b", "main"]);
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "initial"]);
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: rel-nochk\n  name: rel-nochk\n  profile: rust-web\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n",
        )
        .expect("manifest");
        fs::write(
            dir.join("forge.yaml"),
            "schema: 1\nproject:\n  id: rel-nochk\n  name: rel-nochk\n  profile: rust-web\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n",
        )
        .unwrap();
        let config = release_config_from_manifest(&manifest).unwrap();
        let semver = Semver::parse("1.2.3").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        // Doctor needs a forge.yaml; without a changelog the
        // load_changelog call would fail, so create one.
        fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        // Plan is not ready because doctor reports a
        // `build-config` finding (no Cargo.toml in the
        // empty project).
        assert!(!plan.healthy());
        let doctor = plan
            .checks
            .iter()
            .find(|c| c.kind == "doctor")
            .expect("doctor check");
        assert!(doctor.status != "pass" || !plan.ready);
    }

    #[test]
    fn prepare_omits_docs_stage_when_no_locale_configured() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-bare", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("0.1.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: true,
            retry: false,
            stages: config.stages.clone(),
        };
        let policy = DriftWatchConfig::from_env();
        let plan = prepare_release(dir, &manifest, &config, &request, &policy).unwrap();
        assert!(plan.docs_locales.is_empty());
    }

    #[test]
    fn apply_refuses_without_confirm() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-noconf", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: false,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let err = apply_release(dir, &manifest, &config, &request, &adapters).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn apply_records_tag_conflict_on_existing_different_commit() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-conflict", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        // First commit on top of initial.
        fs::write(dir.join("README.md"), "v2\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "second"]);
        // Create a `v1.0.0` tag at the original commit.
        let original = String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(dir)
                .arg("rev-parse")
                .arg("HEAD~1")
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        git(
            dir,
            &["tag", "-a", "v1.0.0", "-m", "pre-existing", &original],
        );
        // Move HEAD to a new commit so the requested revision
        // is different from the tagged one.
        fs::write(dir.join("README.md"), "v3\n").unwrap();
        git(dir, &["add", "--", "README.md"]);
        git(dir, &["commit", "-q", "-m", "third"]);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let tag = report
            .stage_outcomes
            .iter()
            .find(|s| s.stage == "tag")
            .expect("tag stage");
        assert_eq!(tag.status, "conflict");
    }

    #[test]
    fn apply_skip_when_tag_points_at_current_commit() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-skip", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let sha = String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(dir)
                .arg("rev-parse")
                .arg("HEAD")
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string();
        git(dir, &["tag", "-a", "v1.0.0", "-m", "preexisting", &sha]);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            // Override the stage list to just the tag
            // stage so the commit stage does not move
            // HEAD before the tag check.
            stages: vec!["tag".to_string()],
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let tag = report
            .stage_outcomes
            .iter()
            .find(|s| s.stage == "tag")
            .expect("tag stage");
        assert_eq!(tag.status, "skipped");
    }

    #[test]
    fn apply_persists_release_state_for_retry() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-state", "");
        let manifest = read_manifest(dir);
        let config = read_release_config(dir);
        let semver = Semver::parse("1.0.0").unwrap();
        let request = ReleaseRequest {
            project_id: manifest.project.id.clone(),
            version: semver,
            confirm: true,
            dry_run: false,
            retry: false,
            stages: config.stages.clone(),
        };
        let adapters = ReleaseAdapterConfig::from_env();
        let report = apply_release(dir, &manifest, &config, &request, &adapters).unwrap();
        let state_path = std::path::PathBuf::from(&report.state_path);
        assert!(state_path.is_file(), "state path missing: {state_path:?}");
    }

    #[test]
    fn list_reports_zero_entries_for_fresh_project() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-list", "");
        let entries = list_releases(dir, "rel-list").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn maturity_in_manifest_is_preserved() {
        // Sanity: the manifest fixture preserves the
        // maturity field so the release contract can use
        // it for doctor without re-reading the manifest.
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_release_manifest(dir, "rel-maturity", "");
        let (manifest, _) = Manifest::load_from_dir(dir, None).unwrap();
        assert!(matches!(manifest.project.maturity, Some(Maturity::L1)));
    }

    #[test]
    fn run_with_timeout_returns_output_when_adapter_completes() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo receipt-123")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let output =
            run_with_timeout(cmd, std::time::Duration::from_secs(10), "test adapter").unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("receipt-123"));
    }

    #[test]
    fn run_with_timeout_kills_and_reaps_child_on_timeout() {
        let tmp = TempDir::new().unwrap();
        let pid_file = tmp.path().join("adapter.pid");
        // Adapter records its pid, then sleeps past the deadline.
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(format!("echo $$ > {} && exec sleep 30", pid_file.display()))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let start = std::time::Instant::now();
        let err = run_with_timeout(cmd, std::time::Duration::from_millis(300), "test adapter")
            .expect_err("timeout must fail");
        assert!(err.contains("timeout"), "unexpected error: {err}");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "timeout must return promptly"
        );
        let pid_text = std::fs::read_to_string(&pid_file).expect("pid file");
        let pid = pid_text.trim().to_string();
        // The direct child must be terminated and reaped: `kill -0`
        // fails when the process no longer exists.
        std::thread::sleep(std::time::Duration::from_millis(200));
        let probe = Command::new("kill")
            .arg("-0")
            .arg(&pid)
            .output()
            .expect("kill probe");
        assert!(
            !probe.status.success(),
            "timed-out adapter child {pid} is still alive"
        );
    }

    #[test]
    fn run_with_timeout_reports_nonzero_exit_without_success() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo boom >&2; exit 3")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        let output =
            run_with_timeout(cmd, std::time::Duration::from_secs(10), "test adapter").unwrap();
        assert!(
            !output.status.success(),
            "non-zero exit must not read as success"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("boom"));
    }
}
