//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    CapturedChangelog, CapturedCheck, ReleaseConfig, ReleaseIdentity, ReleaseRequest,
    CHECK_DISABLED, CHECK_FAIL, CHECK_PASS, CHECK_STALE, CHECK_UNAVAILABLE,
    RELEASE_CONTRACT_VERSION,
};
use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::doctor::{PolicyConclusion, PolicyVerdict};
use crate::gitops::run_test;
use crate::policy::{run_driftwatch, DriftWatchConfig, PolicyOutcome};
use std::path::Path;

use super::apply::capture_source_revision;
use super::model::PlanReport;

/// The release check kinds a project can declare in
/// `release.checks`. The capture and the readiness judgement
/// share this list so the two cannot drift apart. The Gate check
/// is not in this list: it always applies to a release and is
/// never declared.
pub(super) const CONFIGURABLE_CHECK_KINDS: [&str; 3] = ["doctor", "test", "driftwatch"];

/// The verdict of the release readiness judgement. `ready` is
/// true only when at least one check is applicable and every
/// applicable check passes. `reason` is empty when the release
/// is ready and otherwise names the missing evidence.
pub(super) struct CheckReadiness {
    pub ready: bool,
    pub reason: String,
}

/// Judge whether a release has the evidence it needs. A release
/// is ready only when at least one check is applicable and every
/// applicable check passes: an empty applicable set is the
/// absence of evidence, not a pass. The reason names the check
/// kinds a project can declare when no check is applicable, and
/// names the failing check and its status otherwise.
pub(super) fn check_readiness(checks: &[CapturedCheck]) -> CheckReadiness {
    let applicable: Vec<&CapturedCheck> = checks.iter().filter(|c| c.applicable).collect();
    if applicable.is_empty() {
        return CheckReadiness {
            ready: false,
            reason: format!(
                "no applicable release check; declare at least one of {} in release.checks",
                CONFIGURABLE_CHECK_KINDS.join(", ")
            ),
        };
    }
    let failing: Vec<String> = applicable
        .iter()
        .filter(|c| c.status != CHECK_PASS)
        .map(|c| format!("{}:{}", c.kind, c.status))
        .collect();
    if failing.is_empty() {
        CheckReadiness {
            ready: true,
            reason: String::new(),
        }
    } else {
        CheckReadiness {
            ready: false,
            reason: format!("failing check(s) {}", failing.join(", ")),
        }
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
    let readiness = check_readiness(&checks);
    let note = if readiness.ready {
        format!(
            "release plan ready for `{}`: {} check(s) passed at revision `{}`",
            request.project_id,
            checks.iter().filter(|c| c.status == CHECK_PASS).count(),
            short_revision(&revision)
        )
    } else {
        format!(
            "release plan not ready: {}; release execution blocks before tag or publication",
            readiness.reason
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
        ready: readiness.ready,
    })
}

pub(super) fn load_release_changelog(
    project_dir: &Path,
    config: &ReleaseConfig,
) -> Result<CapturedChangelog, ForgeError> {
    super::super::load_changelog(project_dir, &config.changelog)
}

/// Resolve the documentation locales that the release will
/// translate. When the manifest does not request any locale
/// the release omits the docs stage entirely, matching the
/// R1 boundary scenario. When the manifest lists a locale
/// that is disabled in the manifest's docs section, the
/// release refuses to translate it so a stage never runs
/// against a disabled locale.
pub(super) fn effective_docs_locales(manifest: &Manifest, config: &ReleaseConfig) -> Vec<String> {
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

pub(super) fn effective_stages(config: &ReleaseConfig, request: &ReleaseRequest) -> Vec<String> {
    if !request.stages.is_empty() {
        return request.stages.clone();
    }
    config.stages.clone()
}

pub(super) fn short_revision(rev: &str) -> &str {
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
/// Unconfigured checks are reported as `disabled`. The Gate check
/// is not configurable: it is always captured, always applies and
/// is never disabled.
pub(super) fn collect_captured_checks(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    revision: &str,
    policy: &DriftWatchConfig,
) -> Result<Vec<CapturedCheck>, ForgeError> {
    let mut out: Vec<CapturedCheck> = Vec::new();
    let _ = manifest.project.profile.as_str();
    let configured: Vec<String> = config.checks.iter().map(|c| c.kind.clone()).collect();
    for kind in CONFIGURABLE_CHECK_KINDS {
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
    // The Gate check is not declared; it always applies.
    out.push(run_gate_check(project_dir, manifest, revision));
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
            // The doctor owns the judgement of a policy report; the
            // release lane reads its verdict instead of filtering the
            // report again. The staleness fact is the doctor's own
            // rule, so both surfaces read the same fact.
            let stale =
                crate::doctor::freshness::observation_is_stale_from_report(project_dir, &report);
            let verdict = PolicyVerdict::judge(&report, stale);
            driftwatch_check_from_verdict(&verdict, revision)
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

/// Map the doctor's verdict on one policy report onto the release
/// contract's check statuses. A failing rule, a report that belongs
/// to an older source revision and a report no rule applied to each
/// block the release; a warning does not block, but it is never
/// called healthy. Only a report where every applicable rule passed
/// is called healthy.
fn driftwatch_check_from_verdict(verdict: &PolicyVerdict, revision: &str) -> CapturedCheck {
    let (status, evidence, detail) = match verdict.conclusion {
        PolicyConclusion::Failing => (
            CHECK_FAIL,
            verdict.failing.clone(),
            "driftwatch reports failing rules".to_string(),
        ),
        PolicyConclusion::Stale => (
            CHECK_STALE,
            vec!["driftwatch report belongs to an older source revision".to_string()],
            "driftwatch report is stale relative to the current source revision".to_string(),
        ),
        PolicyConclusion::Unjudged => (
            CHECK_UNAVAILABLE,
            vec![
                "no driftwatch rule applied; there is no policy evidence to release on".to_string(),
            ],
            "driftwatch applied no rule; there is no policy evidence to release on".to_string(),
        ),
        PolicyConclusion::Warning => (
            CHECK_PASS,
            vec![format!(
                "driftwatch warned on rule(s) {}",
                verdict.warning.join(", ")
            )],
            "driftwatch report carries warnings".to_string(),
        ),
        PolicyConclusion::Passed => (
            CHECK_PASS,
            vec!["driftwatch report healthy".to_string()],
            "driftwatch report is healthy".to_string(),
        ),
    };
    CapturedCheck {
        kind: "driftwatch".to_string(),
        status: status.to_string(),
        applicable: true,
        evidence,
        detail,
        source_revision: revision.to_string(),
    }
}

/// The release `gate` check. It always applies and can never be
/// disabled. It asks the shared delivery rule
/// ([`crate::gate::ship_gate::evaluate_ship_gate`]) whether the
/// evidence the project persisted may back this revision, and
/// reports the verdict unchanged: `pass` when the rule allows the
/// revision, `fail` with the rule's own reason otherwise. The
/// release lane keeps its own status vocabulary; the reason is the
/// delivery lane's reason, word for word.
fn run_gate_check(project_dir: &Path, manifest: &Manifest, revision: &str) -> CapturedCheck {
    // The manifest's project id is the id the Gate itself bound the
    // evidence to, so it is the id the evidence must match.
    let status =
        crate::gate::ship_gate::evaluate_ship_gate(project_dir, &manifest.project.id, revision);
    let evidence = match crate::gate::load_latest_evidence(project_dir) {
        Ok(Some(record)) => vec![crate::gate::evidence_summary(&record)],
        // No record: name the directory the evidence belongs in,
        // never a summary of evidence that does not exist.
        Ok(None) => vec![format!("{}/", crate::gate::GATE_EVIDENCE_DIR)],
        Err(err) => vec![err.to_string()],
    };
    if status.satisfied {
        CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_PASS.to_string(),
            applicable: true,
            evidence,
            detail: "the shared delivery rule allows this revision".to_string(),
            source_revision: revision.to_string(),
        }
    } else {
        CapturedCheck {
            kind: "gate".to_string(),
            status: CHECK_FAIL.to_string(),
            applicable: true,
            evidence,
            detail: status.reason,
            source_revision: revision.to_string(),
        }
    }
}
