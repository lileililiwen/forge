//! forge — read-only project status and fleet readiness
//! (`forge-project-status/0.1.0`).
//!
//! This module exposes two session-gated, JSON-only reads over the crate's
//! existing in-process assessment planes — never a shell and never a write:
//!
//! - `GET /v1/admin/projects/{id}/status` projects one **managed** project's
//!   doctor, checker and profile-readiness state.
//! - `GET /v1/admin/status` counts every registered project by that same
//!   overall state so the portal can render a fleet-wide readiness tile.
//!
//! Security boundary (see `.ai-rules/concerns/security.md`):
//! - A project is addressed only by an opaque, validated `id`; its root is
//!   resolved **server-side** by the workbench's [`resolve`], so a request can
//!   never name a filesystem path.
//! - The registry is opened strictly **read-only**
//!   ([`Registry::open_read_only`]), and the reused Core functions
//!   ([`run_doctor`], [`governance::inspect`], [`checker::build_document`],
//!   [`inspect_profile`]) are read-only projections. No subprocess that writes
//!   runs, the DriftWatch policy adapter is not invoked (`policy_outcome` is
//!   `None`), and the native readiness matrix / build / test toolchains are
//!   never called.
//! - No response serializes the registered absolute path, a credential or an
//!   adapter binary: findings are reduced to counts, and any manifest-derived
//!   text is passed through [`redact_local_paths`].

use std::path::Path;

use serde_json::{json, Value};

use super::workbench::{redact_local_paths, resolve, Resolved};
use super::ApiResponse;
use crate::checker::{self, AlertSeverity};
use crate::core::manifest::Manifest;
use crate::doctor::{
    delivery_observation, run_doctor, DoctorReport, FindingStatus, HealthPartition,
    RegistryObservation,
};
use crate::governance;
use crate::profile::{inspect_profile, ProfileSupportStatus};
use crate::registry::{ProjectRecord, Registry};

/// Versioned status contract. Additive only; removing or renaming a stable
/// field requires a contract migration.
pub const CONTRACT_VERSION: &str = "forge-project-status/0.1.0";

/// The two read-only routes this module implements. Exported so the command
/// catalog names the exact paths the router registers.
pub const ROUTE_PROJECT_STATUS: &str = "GET /v1/admin/projects/{id}/status";
pub const ROUTE_FLEET_STATUS: &str = "GET /v1/admin/status";

/// Upper bound on the per-project rows echoed by the fleet summary. Every
/// registered project is still counted; only the detail list is bounded.
const MAX_FLEET_STATUS_PROJECTS: usize = 200;

/// Shared non-live note: this surface runs no adapter and no native toolchain.
const NOTE: &str = "read-only in-process status; the DriftWatch policy adapter and native build/test toolchains are not run.";

const STATE_HEALTHY: &str = "healthy";
const STATE_ISSUES: &str = "issues";
const STATE_STALE: &str = "stale";
const STATE_UNAVAILABLE: &str = "unavailable";

/// One projected sub-check. `reason` is empty for a healthy/stale/issues
/// check whose inputs did run; it is set only when the check could not run,
/// and is always a safe, path-free phrase.
struct StatusCheck {
    id: &'static str,
    state: &'static str,
    summary: String,
    reason: String,
}

impl StatusCheck {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "state": self.state,
            "summary": self.summary,
            "reason": self.reason,
        })
    }
}

/// The projected status of one project: its three sub-checks and the reduced
/// overall state.
struct ProjectStatus {
    checks: Vec<StatusCheck>,
    overall: &'static str,
}

/// `GET /v1/admin/projects/{id}/status`: read-only status of one managed
/// project, reused by the workbench. The registry is opened read-only and the
/// root comes only from the server-side [`resolve`].
pub fn project_status(db_path: &Path, id: &str) -> ApiResponse {
    let registry = match Registry::open_read_only(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let record = match resolve(&registry, id) {
        Resolved::Managed(record) => record,
        Resolved::Refused { status, reason } => return refuse(status, reason),
    };
    let status = evaluate(&registry, &record);
    ApiResponse::json(
        200,
        json!({
            "contract": CONTRACT_VERSION,
            "project_id": record.id,
            "management": management_label(&record.id),
            "observed_at": record.observed_at,
            "live": false,
            "state": status.overall,
            "note": NOTE,
            "checks": status.checks.iter().map(StatusCheck::to_json).collect::<Vec<_>>(),
        }),
    )
}

/// `GET /v1/admin/status`: the fleet readiness summary. Counts every
/// registered project by its overall state and returns a bounded sample of the
/// per-project states. Non-live: no adapter, no build, no fixture.
pub fn fleet_status(db_path: &Path) -> ApiResponse {
    let registry = match Registry::open_read_only(db_path) {
        Ok(registry) => registry,
        Err(_) => return unavailable(),
    };
    let records = match registry.list() {
        Ok(records) => records,
        Err(_) => return unavailable(),
    };

    let mut counts = Counts::default();
    let mut projects: Vec<Value> = Vec::new();
    for record in &records {
        let overall = evaluate(&registry, record).overall;
        counts.add(overall);
        if projects.len() < MAX_FLEET_STATUS_PROJECTS {
            projects.push(json!({ "project_id": record.id, "state": overall }));
        }
    }
    let total = records.len();
    let truncated = projects.len() < total;

    let mut body = json!({
        "contract": CONTRACT_VERSION,
        "live": false,
        "generated_at": checker::now_rfc3339(),
        "total": total,
        "counts": {
            "healthy": counts.healthy,
            "issues": counts.issues,
            "stale": counts.stale,
            "unavailable": counts.unavailable,
        },
        "projects": projects,
    });
    if truncated {
        body["truncated"] = json!(true);
    }
    ApiResponse::json(200, body)
}

/// Per-state tallies for the fleet summary.
#[derive(Default)]
struct Counts {
    healthy: usize,
    issues: usize,
    stale: usize,
    unavailable: usize,
}

impl Counts {
    fn add(&mut self, state: &str) {
        match state {
            STATE_HEALTHY => self.healthy += 1,
            STATE_ISSUES => self.issues += 1,
            STATE_STALE => self.stale += 1,
            _ => self.unavailable += 1,
        }
    }
}

/// Project one record's three sub-checks and reduce them. An unreadable root
/// short-circuits to three `unavailable` checks without calling Core at all.
/// The delivery state is read through the delivery lane's own projection, so
/// this surface reports the same delivery finding as the CLI.
fn evaluate(registry: &Registry, record: &ProjectRecord) -> ProjectStatus {
    let dir = Path::new(&record.path);
    if !record.available || !dir.is_dir() {
        let reason = "the registered project directory is not currently readable on this host; nothing was changed.";
        return ProjectStatus {
            checks: vec![
                StatusCheck {
                    id: "doctor",
                    state: STATE_UNAVAILABLE,
                    summary: String::new(),
                    reason: reason.to_string(),
                },
                StatusCheck {
                    id: "check",
                    state: STATE_UNAVAILABLE,
                    summary: String::new(),
                    reason: reason.to_string(),
                },
                StatusCheck {
                    id: "readiness",
                    state: STATE_UNAVAILABLE,
                    summary: String::new(),
                    reason: reason.to_string(),
                },
            ],
            overall: STATE_UNAVAILABLE,
        };
    }

    let observation = RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at.clone()),
        delivery: Some(delivery_observation(
            registry,
            &record.id,
            chrono::Utc::now(),
        )),
    };
    // `policy_outcome = None`: the DriftWatch adapter is never invoked by this
    // read-only surface, so no external process is started.
    let report = run_doctor(dir, None, Some(&observation), None).ok();

    let doctor = doctor_check(report.as_ref());
    let check = check_check(dir, report.as_ref());
    let readiness = readiness_check(dir);
    let overall = overall_state(doctor.state, check.state, readiness.state);
    ProjectStatus {
        checks: vec![doctor, check, readiness],
        overall,
    }
}

/// Map the doctor report to a sub-check state.
///
/// The doctor's own health partition is the rule: a report that carries
/// blocking evidence is `issues`, a report that carries none but whose
/// observation is stale is `stale`, and a report that carries none and is
/// current is `healthy`. The partition is the inverse of the doctor's own
/// `healthy` field — a report the doctor does not call healthy is never
/// projected as healthy, and a report it calls healthy is never projected as
/// anything else.
///
/// The summary names every kind of problem apart, so a check that could not
/// run is never counted as a failing check and the summary can never report
/// no problems while the state reports issues.
fn doctor_check(report: Option<&DoctorReport>) -> StatusCheck {
    let Some(report) = report else {
        return StatusCheck {
            id: "doctor",
            state: STATE_UNAVAILABLE,
            summary: String::new(),
            reason: "the health inspection could not run; nothing was changed.".to_string(),
        };
    };
    let blocking = report.blocking_findings();
    let failed = blocking
        .iter()
        .filter(|finding| finding.status == FindingStatus::Fail)
        .count();
    let unavailable = blocking
        .iter()
        .filter(|finding| finding.status == FindingStatus::Unavailable)
        .count();
    let unmet = report.unmet_controls().len();
    match report.health_partition() {
        HealthPartition::Issues => {
            let mut summary = format!(
                "{failed} failing finding(s), {unavailable} unavailable finding(s), {unmet} unmet control(s)"
            );
            if report.stale {
                summary.push_str("; the registry observation is stale");
            }
            StatusCheck {
                id: "doctor",
                state: STATE_ISSUES,
                summary,
                reason: String::new(),
            }
        }
        HealthPartition::Stale => StatusCheck {
            id: "doctor",
            state: STATE_STALE,
            summary:
                "the registry observation predates the current manifest; re-register to refresh"
                    .to_string(),
            reason: String::new(),
        },
        HealthPartition::Healthy => StatusCheck {
            id: "doctor",
            state: STATE_HEALTHY,
            summary: format!(
                "{} finding(s), {} unmet control(s); no failing evidence",
                report.findings.len(),
                unmet
            ),
            reason: String::new(),
        },
    }
}

/// Map the in-process checker document to a sub-check state. It reuses the
/// exact CLI assembly over the doctor report and the read-only governance
/// observation; any alert (error or warning) is `issues`. Only alert counts
/// are surfaced, never the alert text.
fn check_check(dir: &Path, report: Option<&DoctorReport>) -> StatusCheck {
    let Some(report) = report else {
        return StatusCheck {
            id: "check",
            state: STATE_UNAVAILABLE,
            summary: String::new(),
            reason: "no health report was produced, so the check projection could not run; nothing was changed."
                .to_string(),
        };
    };
    let governance = governance::inspect(dir);
    let document = checker::build_document(
        dir,
        report,
        &governance,
        &checker::now_rfc3339(),
        checker::DEFAULT_MAX_ALERTS,
    );
    let errors = document
        .alerts
        .iter()
        .filter(|alert| alert.severity == AlertSeverity::Error)
        .count();
    let warnings = document.alerts.len() - errors;
    if errors == 0 && warnings == 0 {
        StatusCheck {
            id: "check",
            state: STATE_HEALTHY,
            summary: "no error or warning evidence".to_string(),
            reason: String::new(),
        }
    } else {
        StatusCheck {
            id: "check",
            state: STATE_ISSUES,
            summary: format!("{errors} error(s), {warnings} warning(s)"),
            reason: String::new(),
        }
    }
}

/// Map the read-only profile/readiness projection to a sub-check state. The
/// native readiness matrix is never invoked: only the descriptor's catalog
/// support status is read.
fn readiness_check(dir: &Path) -> StatusCheck {
    let profile = match Manifest::load_from_dir(dir, None) {
        Ok((manifest, _)) => manifest.project.profile,
        Err(_) => {
            return StatusCheck {
                id: "readiness",
                state: STATE_UNAVAILABLE,
                summary: String::new(),
                reason: "the manifest could not be read, so profile readiness cannot be evaluated; nothing was changed."
                    .to_string(),
            };
        }
    };
    let profile = redact_local_paths(&profile);
    match inspect_profile(&profile) {
        Ok(descriptor) if descriptor.support_status == ProfileSupportStatus::Supported => StatusCheck {
            id: "readiness",
            state: STATE_HEALTHY,
            summary: format!("profile `{profile}` has certified native-template support"),
            reason: String::new(),
        },
        Ok(descriptor) if descriptor.support_status == ProfileSupportStatus::Native => StatusCheck {
            id: "readiness",
            state: STATE_HEALTHY,
            summary: format!(
                "profile `{profile}` is a native project stack maintained and built natively"
            ),
            reason: String::new(),
        },
        Ok(_) => StatusCheck {
            id: "readiness",
            state: STATE_ISSUES,
            summary: format!(
                "profile `{profile}` is a planned catalog candidate with no certified native-template evidence"
            ),
            reason: String::new(),
        },
        Err(_) => StatusCheck {
            id: "readiness",
            state: STATE_ISSUES,
            summary: format!("profile `{profile}` is not a known catalog profile"),
            reason: String::new(),
        },
    }
}

/// Deterministic reduction: `issues` > `unavailable` > `stale` > `healthy`. A
/// real finding is never hidden behind missing evidence, and a genuinely
/// all-healthy project is `healthy`.
fn overall_state(doctor: &str, check: &str, readiness: &str) -> &'static str {
    let states = [doctor, check, readiness];
    if states.contains(&STATE_ISSUES) {
        STATE_ISSUES
    } else if states.contains(&STATE_UNAVAILABLE) {
        STATE_UNAVAILABLE
    } else if states.contains(&STATE_STALE) {
        STATE_STALE
    } else {
        STATE_HEALTHY
    }
}

/// `self` for the special Forge project, `managed` otherwise. Mirrors the
/// workbench's label so both projections agree.
fn management_label(id: &str) -> &'static str {
    if id == "forge" {
        "self"
    } else {
        "managed"
    }
}

fn refuse(status: u16, reason: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({
            "contract": CONTRACT_VERSION,
            "error": { "code": code_for(status), "message": reason },
            "effect": "none",
        }),
    )
}

fn unavailable() -> ApiResponse {
    refuse(
        503,
        "the project status service could not read the registry; no files, registry rows or journal entries were changed.",
    )
}

fn code_for(status: u16) -> &'static str {
    match status {
        400 => "project-status-invalid",
        404 => "project-status-unmanaged-project",
        _ => "project-status-unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::{Finding, MaturityControl, Remediation};
    use std::fs;

    /// One applicable finding of the given status.
    fn finding(id: &str, status: FindingStatus) -> Finding {
        Finding {
            id: id.to_string(),
            status,
            evidence: vec![format!("{id} evidence")],
            applicable: true,
            remediation: Remediation::Manual,
            detail: format!("{id} detail"),
        }
    }

    /// One applicable control without satisfying evidence.
    fn unmet_control(id: &str) -> MaturityControl {
        MaturityControl {
            id: id.to_string(),
            level: "L1".to_string(),
            description: format!("{id} control"),
            applicable: true,
            met: false,
            evidence: vec!["no evidence was gathered".to_string()],
        }
    }

    /// A report whose `healthy` field follows the doctor's own rule, so the
    /// projection can be compared with the field it must invert.
    fn report(findings: Vec<Finding>, controls: Vec<MaturityControl>, stale: bool) -> DoctorReport {
        let unmet = controls.iter().filter(|c| c.applicable && !c.met).count();
        let healthy = findings
            .iter()
            .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
            && unmet == 0
            && !stale;
        DoctorReport {
            path: ".".to_string(),
            profile: None,
            policy_version: "0.1.0".to_string(),
            current_maturity: Some("L1".to_string()),
            target_maturity: Some("L1".to_string()),
            findings,
            controls,
            healthy,
            stale,
        }
    }

    /// The count a summary names for one kind of problem, e.g.
    /// `unavailable finding(s)`.
    fn count_named(summary: &str, label: &str) -> usize {
        summary
            .split(", ")
            .find(|part| part.contains(label))
            .and_then(|part| part.split(' ').next())
            .and_then(|digits| digits.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("the summary {summary:?} must name the {label} count"))
    }

    #[test]
    fn an_unavailable_finding_is_issues_and_names_the_unavailable_count() {
        let report = report(
            vec![finding("repository", FindingStatus::Unavailable)],
            Vec::new(),
            false,
        );
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(
            check.summary,
            "0 failing finding(s), 1 unavailable finding(s), 0 unmet control(s)"
        );
        assert!(
            !check
                .summary
                .contains("0 failing finding(s), 0 unmet control(s)"),
            "a summary must never report no problems while its state reports issues"
        );
    }

    #[test]
    fn a_failing_finding_is_issues_and_names_the_failing_count() {
        let report = report(
            vec![finding("profile-known", FindingStatus::Fail)],
            Vec::new(),
            false,
        );
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(count_named(&check.summary, "failing finding(s)"), 1);
        assert_eq!(count_named(&check.summary, "unavailable finding(s)"), 0);
    }

    #[test]
    fn a_failing_and_an_unavailable_finding_are_named_apart() {
        let report = report(
            vec![
                finding("profile-known", FindingStatus::Fail),
                finding("repository", FindingStatus::Unavailable),
            ],
            Vec::new(),
            false,
        );
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(count_named(&check.summary, "failing finding(s)"), 1);
        assert_eq!(count_named(&check.summary, "unavailable finding(s)"), 1);
    }

    #[test]
    fn an_unmet_control_is_issues_and_names_the_unmet_count() {
        let report = report(Vec::new(), vec![unmet_control("L1-structure")], false);
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(count_named(&check.summary, "unmet control(s)"), 1);
    }

    #[test]
    fn a_healthy_report_keeps_its_wording() {
        let report = report(
            vec![
                finding("manifest-valid", FindingStatus::Pass),
                finding("docs-present", FindingStatus::Warn),
            ],
            Vec::new(),
            false,
        );
        assert!(report.healthy);
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_HEALTHY);
        assert_eq!(
            check.summary,
            "2 finding(s), 0 unmet control(s); no failing evidence"
        );
    }

    #[test]
    fn a_stale_report_with_nothing_else_wrong_is_stale() {
        let report = report(
            vec![finding("manifest-valid", FindingStatus::Pass)],
            Vec::new(),
            true,
        );
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_STALE);
        assert_eq!(
            check.summary,
            "the registry observation predates the current manifest; re-register to refresh"
        );
    }

    #[test]
    fn a_stale_report_with_an_unavailable_finding_is_issues_and_names_the_staleness() {
        let report = report(
            vec![finding("repository", FindingStatus::Unavailable)],
            Vec::new(),
            true,
        );
        let check = doctor_check(Some(&report));
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(
            check.summary,
            "0 failing finding(s), 1 unavailable finding(s), 0 unmet control(s); the registry observation is stale"
        );
    }

    #[test]
    fn the_doctor_and_check_sub_checks_count_the_same_unavailable_findings() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(
            tmp.path().join("forge.yaml"),
            "schema: 1\nproject:\n  id: agreement\n  name: agreement\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
        )
        .unwrap();
        let report = report(
            vec![finding("repository", FindingStatus::Unavailable)],
            Vec::new(),
            false,
        );

        let doctor = doctor_check(Some(&report));
        let check = check_check(tmp.path(), Some(&report));
        assert_eq!(doctor.state, STATE_ISSUES);
        assert_eq!(check.state, STATE_ISSUES);
        assert_eq!(count_named(&doctor.summary, "unavailable finding(s)"), 1);
        assert_eq!(
            count_named(&check.summary, "warning(s)"),
            count_named(&doctor.summary, "unavailable finding(s)"),
            "the two sub-checks must count the same unavailable findings: doctor={:?} check={:?}",
            doctor.summary,
            check.summary
        );
    }
}
