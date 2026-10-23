//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{Manifest, Maturity};
use crate::policy::{redact_report_in_place, PolicyOutcome, PolicyReport, PolicySeverity};
use std::path::Path;

use super::freshness::{marker_files_present, observation_is_stale_from_report};
use super::model::{
    DoctorReport, Finding, FindingStatus, GitState, MaturityControl, RegistryObservation,
    Remediation, Rollup,
};
use super::probes::{
    detect_ci, detect_deployment, detect_driftwatch, file_exists, has_admin_markers,
    has_audit_markers, has_auth_markers, has_build_definition, has_database_markers,
    has_observability_markers, maturity_rank, probe_git_remote, requests_db,
    BuildDefinitionOutcome,
};
use super::runner::control;
/// Evidence-based maturity assessment against the target level. Controls
/// above the target are recorded as nonapplicable rather than forced;
/// database/auth-gated controls are nonapplicable when the project does
/// not request them, so L0 prototypes are respected.
pub(super) fn assess_maturity(
    dir: &Path,
    manifest: &Manifest,
    descriptor: Option<&crate::profile::ProfileDescriptor>,
    deps: &str,
    target: Maturity,
    observation: Option<&RegistryObservation>,
) -> Vec<MaturityControl> {
    let rank = maturity_rank(target);
    let at_least = |level: Maturity| rank >= maturity_rank(level);
    let profile_id = manifest.project.profile.as_str();
    let capabilities: Vec<&str> = descriptor
        .map(|d| d.capabilities.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let supports = |cap: &str| capabilities.contains(&cap);

    // The build presence reads the doctor's three-state outcome: present
    // meets the L1 build control, missing leaves it unmet with the missing
    // evidence, and a profile the doctor cannot judge leaves it unmet with
    // the profile named. A profile the catalogue does not know keeps the
    // `unknown profile` evidence.
    let (build_present, build_evidence) =
        match descriptor.map(|_| has_build_definition(dir, profile_id)) {
            None => (false, vec!["unknown profile".to_string()]),
            Some(BuildDefinitionOutcome::Present(found)) => (true, found),
            Some(BuildDefinitionOutcome::Missing(missing)) => (false, missing),
            Some(BuildDefinitionOutcome::UnknownProfile) => (
                false,
                vec![format!(
                    "the doctor has no expected build definition for profile '{profile_id}'"
                )],
            ),
        };
    let structure_met = build_present && file_exists(dir, "forge.yaml");
    let structure_evidence = if structure_met {
        vec!["forge.yaml and profile build definition are present".to_string()]
    } else {
        build_evidence.clone()
    };

    let want_db = requests_db(&manifest.features);
    let db_applicable = at_least(Maturity::L1)
        && want_db
        && descriptor.map(|d| d.requires_database).unwrap_or(false);
    let (db_met, db_evidence) = has_database_markers(deps);

    let want_logging = manifest.features.contains_key("telemetry")
        || manifest.features.contains_key("analytics")
        || manifest.features.contains_key("logging");
    let (obs_met, obs_evidence) = has_observability_markers(deps, &manifest.features);

    let want_health = manifest.features.contains_key("health-check");
    let health_met =
        deps.contains("health") || dir.join("src/health").exists() || want_health && build_present;
    let health_evidence =
        if health_met && (deps.contains("health") || dir.join("src/health").exists()) {
            vec!["health markers found".to_string()]
        } else if want_health && build_present {
            vec![
                "health-check declared; build definition present but no health markers found"
                    .to_string(),
            ]
        } else {
            vec!["no health markers found".to_string()]
        };

    let (auth_met, auth_evidence) = has_auth_markers(deps, dir);
    let (admin_met, admin_evidence) = has_admin_markers(deps, dir, &manifest.features);
    let (audit_met, audit_evidence) = has_audit_markers(deps, &manifest.features);
    let (ci_present, ci_evidence) = detect_ci(dir);
    let (dw_present, dw_evidence) = detect_driftwatch(dir);
    let (deploy_present, deploy_evidence) = detect_deployment(dir);

    // The registration state is only known when the caller supplied an
    // observation. Without one the control says so and stays not
    // applicable, so the project is never failed for a fact the doctor
    // never read.
    let registration_known = observation.is_some();
    let registered = observation.map(|o| o.registered).unwrap_or(false);
    let distribution_met = manifest
        .distribution
        .as_ref()
        .is_some_and(|d| d.primary.is_some() || !d.mirrors.is_empty());
    let distribution_evidence = match &manifest.distribution {
        Some(d) if d.primary.is_some() || !d.mirrors.is_empty() => {
            vec!["manifest distribution declares primary or mirrors".to_string()]
        }
        _ => vec!["manifest declares no distribution primary or mirrors".to_string()],
    };
    let release_evidence = marker_files_present(
        dir,
        &["CHANGELOG.md", "RELEASES.md", "release.yaml", "release.yml"],
        &[],
    );
    let backup_evidence = marker_files_present(
        dir,
        &["BACKUP.md", "backup.yaml", "backup.yml"],
        &["backup"],
    );
    let recovery_evidence = marker_files_present(
        dir,
        &[
            "RECOVERY.md",
            "DISASTER_RECOVERY.md",
            "recovery.yaml",
            "restore.sh",
        ],
        &["recovery", "restore"],
    );
    let monitoring_evidence = marker_files_present(
        dir,
        &[
            "prometheus.yml",
            "grafana.yaml",
            "monitoring.yaml",
            "MONITORING.md",
        ],
        &["monitoring", "grafana"],
    );
    let security_evidence = marker_files_present(dir, &["SECURITY.md", "security.yaml"], &[]);
    let privacy_evidence = marker_files_present(dir, &["PRIVACY.md", "privacy.yaml"], &[]);
    let secrets_evidence = marker_files_present(
        dir,
        &[".sops.yaml", ".secrets.yaml", "vault.yaml", "SECRETS.md"],
        &["secrets", "sealed-secrets"],
    );
    let alerts_evidence = marker_files_present(
        dir,
        &["alertmanager.yml", "alerts.yaml", "ALERTS.md"],
        &["alerts"],
    );

    // Upgrade evidence: version-controlled automation that can repeat a
    // migration (git remote + CI). Recorded explicitly, never inferred.
    let upgrades_met = matches!(probe_git_remote(dir), GitState::Detected(_)) && ci_present;
    let upgrades_evidence = if upgrades_met {
        vec!["git remote and ci configuration provide a repeatable upgrade path".to_string()]
    } else {
        vec!["no combined git-remote and ci evidence for repeatable upgrades".to_string()]
    };

    vec![
        control(
            "L1-configuration",
            Maturity::L1,
            "manifest is schema-valid (L1 configuration)",
            at_least(Maturity::L1),
            true,
            vec!["forge.yaml parses against schema 1".to_string()],
        ),
        control(
            "L1-structure",
            Maturity::L1,
            "the project carries its manifest and its profile's build definition (L1 structure)",
            at_least(Maturity::L1),
            structure_met,
            structure_evidence,
        ),
        control(
            "L1-database",
            Maturity::L1,
            "database evidence where a database is requested (L1 database)",
            db_applicable,
            db_met,
            db_evidence,
        ),
        control(
            "L1-logging",
            Maturity::L1,
            "logging/telemetry evidence where requested (L1 logging)",
            at_least(Maturity::L1) && want_logging,
            obs_met,
            obs_evidence.clone(),
        ),
        control(
            "L1-health",
            Maturity::L1,
            "health evidence where a health check is requested (L1 health)",
            at_least(Maturity::L1) && want_health,
            health_met && (deps.contains("health") || dir.join("src/health").exists()),
            health_evidence,
        ),
        control(
            "L1-build",
            Maturity::L1,
            "build definition is present (L1 build definition)",
            at_least(Maturity::L1),
            build_present,
            build_evidence,
        ),
        control(
            "L2-auth",
            Maturity::L2,
            "auth evidence where the profile supports auth (L2 auth)",
            at_least(Maturity::L2) && supports("auth"),
            auth_met,
            auth_evidence,
        ),
        control(
            "L2-admin",
            Maturity::L2,
            "admin evidence where the profile supports admin (L2 admin)",
            at_least(Maturity::L2) && supports("admin"),
            admin_met,
            admin_evidence,
        ),
        control(
            "L2-ci",
            Maturity::L2,
            "continuous integration is configured (L2 CI)",
            at_least(Maturity::L2),
            ci_present,
            ci_evidence,
        ),
        control(
            "L2-driftwatch",
            Maturity::L2,
            "driftwatch is configured; execution belongs to v0.3 (L2 DriftWatch)",
            at_least(Maturity::L2),
            dw_present,
            dw_evidence,
        ),
        control(
            "L2-deployment",
            Maturity::L2,
            "deployment automation is configured (L2 deployment)",
            at_least(Maturity::L2),
            deploy_present,
            deploy_evidence,
        ),
        control(
            "L2-audit",
            Maturity::L2,
            "audit evidence where the profile supports audit (L2 audit)",
            at_least(Maturity::L2) && supports("audit"),
            audit_met,
            audit_evidence,
        ),
        control(
            "L3-identity-compat",
            Maturity::L3,
            "identity compatibility evidence where auth applies (L3 identity compatibility)",
            at_least(Maturity::L3) && supports("auth"),
            auth_met,
            vec!["same evidence as L2-auth".to_string()],
        ),
        control(
            "L3-observability",
            Maturity::L3,
            "observability evidence (L3 observability)",
            at_least(Maturity::L3),
            obs_met,
            obs_evidence,
        ),
        control(
            "L3-release",
            Maturity::L3,
            "release evidence such as a changelog (L3 release)",
            at_least(Maturity::L3),
            release_evidence.0,
            release_evidence.1,
        ),
        control(
            "L3-registry",
            Maturity::L3,
            "project is registered (L3 registry)",
            at_least(Maturity::L3) && registration_known,
            registered,
            if registration_known {
                vec![if registered {
                    "project is registered".to_string()
                } else {
                    "project is not registered".to_string()
                }]
            } else {
                vec![
                    "no registry observation was supplied; the registration state is unknown to the doctor"
                        .to_string(),
                ]
            },
        ),
        control(
            "L3-distribution",
            Maturity::L3,
            "distribution primary or mirrors declared (L3 distribution)",
            at_least(Maturity::L3),
            distribution_met,
            distribution_evidence,
        ),
        control(
            "L3-upgrades",
            Maturity::L3,
            "repeatable upgrade path via git remote and CI (L3 upgrades)",
            at_least(Maturity::L3),
            upgrades_met,
            upgrades_evidence,
        ),
        control(
            "L4-backup",
            Maturity::L4,
            "backup evidence (L4 backup)",
            at_least(Maturity::L4),
            backup_evidence.0,
            backup_evidence.1,
        ),
        control(
            "L4-recovery",
            Maturity::L4,
            "recovery evidence (L4 recovery)",
            at_least(Maturity::L4),
            recovery_evidence.0,
            recovery_evidence.1,
        ),
        control(
            "L4-monitoring",
            Maturity::L4,
            "monitoring evidence (L4 monitoring)",
            at_least(Maturity::L4),
            monitoring_evidence.0,
            monitoring_evidence.1,
        ),
        control(
            "L4-security",
            Maturity::L4,
            "security policy evidence (L4 security)",
            at_least(Maturity::L4),
            security_evidence.0,
            security_evidence.1,
        ),
        control(
            "L4-privacy",
            Maturity::L4,
            "privacy policy evidence (L4 privacy)",
            at_least(Maturity::L4),
            privacy_evidence.0,
            privacy_evidence.1,
        ),
        control(
            "L4-secrets",
            Maturity::L4,
            "secrets management evidence (L4 secrets)",
            at_least(Maturity::L4),
            secrets_evidence.0,
            secrets_evidence.1,
        ),
        control(
            "L4-alerts",
            Maturity::L4,
            "alerting evidence (L4 alerts)",
            at_least(Maturity::L4),
            alerts_evidence.0,
            alerts_evidence.1,
        ),
    ]
}

/// Render a report for human CLI output.
pub fn render_report_human(report: &DoctorReport) -> String {
    let mut lines = vec![
        format!("doctor: {}", report.path),
        format!(
            "profile: {}",
            report.profile.as_deref().unwrap_or("unknown")
        ),
        format!(
            "maturity: current {} -> target {} (policy {})",
            report.current_maturity.as_deref().unwrap_or("unknown"),
            report.target_maturity.as_deref().unwrap_or("unknown"),
            report.policy_version,
        ),
        format!(
            "verdict: {}",
            if report.healthy {
                "healthy"
            } else if report.stale {
                "stale (re-run forge register to refresh)"
            } else {
                "not healthy"
            }
        ),
        String::new(),
        "Findings:".to_string(),
    ];
    for finding in &report.findings {
        lines.push(format!(
            "  [{}] {} ({})",
            finding.status, finding.id, finding.remediation
        ));
        for evidence in &finding.evidence {
            lines.push(format!("    evidence: {evidence}"));
        }
        lines.push(format!("    detail: {}", finding.detail));
        if !finding.applicable {
            lines.push("    applicability: not applicable".to_string());
        }
    }
    lines.push(String::new());
    lines.push("Maturity controls:".to_string());
    for c in &report.controls {
        let state = if !c.applicable {
            "N/A"
        } else if c.met {
            "met"
        } else {
            "UNMET"
        };
        lines.push(format!(
            "  [{state}] {} ({}): {}",
            c.id, c.level, c.description
        ));
        for evidence in &c.evidence {
            lines.push(format!("    evidence: {evidence}"));
        }
    }
    lines.join("\n")
}

/// The doctor's conclusion about one policy report. The precedence is
/// the doctor's own and it is unchanged: an applicable rule that
/// failed outranks a report that belongs to an older source state, a
/// stale report outranks a report no rule applied to, an unjudged
/// report outranks a warning, and a warning outranks a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyConclusion {
    /// An applicable rule failed.
    Failing,
    /// The report belongs to an older source state than the one judged.
    Stale,
    /// No rule applied, so nothing was judged.
    Unjudged,
    /// Applicable rules warned; none failed.
    Warning,
    /// Every applicable rule passed.
    Passed,
}
impl PolicyConclusion {
    /// The doctor's finding status for this conclusion. A stale
    /// report reads as a warning and an unjudged report reads as
    /// unavailable — never as a pass.
    pub fn to_finding_status(self) -> FindingStatus {
        match self {
            PolicyConclusion::Failing => FindingStatus::Fail,
            PolicyConclusion::Stale => FindingStatus::Warn,
            PolicyConclusion::Unjudged => FindingStatus::Unavailable,
            PolicyConclusion::Warning => FindingStatus::Warn,
            PolicyConclusion::Passed => FindingStatus::Pass,
        }
    }
}
/// The doctor's judgement of one policy report: the conclusion and
/// the rule ids behind it. The verdict is the single rule every
/// consumer of a policy report reads, so the release lane and the
/// doctor cannot disagree about the same report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyVerdict {
    /// What the report is.
    pub conclusion: PolicyConclusion,
    /// Ids of the applicable rules that failed.
    pub failing: Vec<String>,
    /// Ids of the applicable rules that warned.
    pub warning: Vec<String>,
    /// How many rules applied to the profile.
    pub applicable: usize,
    /// How many applicable rules passed.
    pub passed: usize,
    /// Whether the report belongs to an older source state.
    pub stale: bool,
}
impl PolicyVerdict {
    /// Judge one policy report with the doctor's precedence. `stale`
    /// is the staleness fact read from the report's own source
    /// revision, so every consumer reaches the same fact.
    pub fn judge(report: &PolicyReport, stale: bool) -> Self {
        let mut failing: Vec<String> = Vec::new();
        let mut warning: Vec<String> = Vec::new();
        let mut applicable = 0usize;
        let mut passed = 0usize;
        for pf in &report.findings {
            if !pf.applicable {
                continue;
            }
            applicable += 1;
            match pf.severity {
                PolicySeverity::Pass => passed += 1,
                PolicySeverity::Warn => warning.push(pf.id.clone()),
                PolicySeverity::Fail => failing.push(pf.id.clone()),
            }
        }
        let conclusion = if !failing.is_empty() {
            PolicyConclusion::Failing
        } else if stale {
            PolicyConclusion::Stale
        } else if applicable == 0 {
            PolicyConclusion::Unjudged
        } else if !warning.is_empty() {
            PolicyConclusion::Warning
        } else {
            PolicyConclusion::Passed
        };
        PolicyVerdict {
            conclusion,
            failing,
            warning,
            applicable,
            passed,
            stale,
        }
    }
}

/// Convert a DriftWatch policy outcome into one or more typed findings
/// while preserving the original rule ID, category, severity, tool
/// version and (already redacted) evidence. A missing binary, non-zero
/// exit, timeout or unparseable payload all surface as a single
/// `driftwatch-policy` finding with status `unavailable`; never as
/// `pass`. Findings reported with `applicable == false` are surfaced
/// separately so a future detector does not invent a result.
pub(super) fn policy_findings(dir: &Path, outcome: Option<&PolicyOutcome>) -> Vec<Finding> {
    let mut out = Vec::new();
    let Some(outcome) = outcome else {
        return out;
    };
    match outcome {
        PolicyOutcome::Unavailable { reason } => {
            out.push(Finding::new(
                "driftwatch-policy",
                FindingStatus::Unavailable,
                vec![format!("adapter did not produce a report: {reason}")],
                true,
                Remediation::Manual,
                "delegated policy execution is unavailable; see evidence for the reason",
            ));
        }
        PolicyOutcome::Reported(report) => {
            out.extend(reported_policy_findings(dir, report));
        }
    }
    out
}

fn reported_policy_findings(dir: &Path, report: &PolicyReport) -> Vec<Finding> {
    let mut safe_report = report.clone();
    // Defense in depth: every evidence line passes through the
    // redaction pipeline even if the adapter already ran it, so a
    // report constructed by a different caller cannot leak
    // credentials into storage or display.
    redact_report_in_place(&mut safe_report);
    let report = &safe_report;
    let mut out = Vec::new();
    let tool_version = report.tool_version.clone();
    let stale = observation_is_stale_from_report(dir, report);
    for pf in &report.findings {
        if !pf.applicable {
            out.push(Finding::new(
                format!("driftwatch-{}", pf.id).as_str(),
                FindingStatus::Pass,
                vec![format!(
                    "tool {} {} reports policy not applicable: {}",
                    report.tool,
                    tool_version,
                    pf.reason.clone().unwrap_or_else(|| pf.message.clone())
                )],
                false,
                Remediation::Manual,
                "policy is not applicable to this profile; reason preserved",
            ));
            continue;
        }
        let status = match pf.severity {
            PolicySeverity::Pass => FindingStatus::Pass,
            PolicySeverity::Warn => FindingStatus::Warn,
            PolicySeverity::Fail => FindingStatus::Fail,
        };
        let mut evidence: Vec<String> = pf
            .evidence
            .iter()
            .map(|line| format!("{} {}: {}", report.tool, tool_version, line))
            .collect();
        if stale {
            evidence.push(format!(
                "observation source is older than current {}; stale",
                crate::core::manifest::CANONICAL_MANIFEST
            ));
        }
        out.push(Finding::new(
            format!("driftwatch-{}", pf.id).as_str(),
            if stale && matches!(status, FindingStatus::Pass) {
                FindingStatus::Warn
            } else {
                status
            },
            evidence,
            true,
            Remediation::Manual,
            format!(
                "[{}:{}] {}",
                pf.category,
                pf.severity.severity_label(),
                pf.message
            ),
        ));
    }
    // Aggregate rollup so the doctor verdict reflects DriftWatch's
    // overall outcome, matching the typed finding contract.
    let rollup = aggregate_rollup(report, stale);
    out.push(Finding::new(
        "driftwatch-policy",
        rollup.status,
        rollup.evidence,
        true,
        Remediation::Manual,
        rollup.detail,
    ));
    out
}

fn aggregate_rollup(report: &PolicyReport, stale: bool) -> Rollup {
    // The rollup is the doctor's own rule, expressed through the
    // shared verdict: the status, the counts and the wording stay
    // exactly as they were.
    let verdict = PolicyVerdict::judge(report, stale);
    let detail = format!(
        "driftwatch {} (contract {}) reported {} finding(s): {} pass, {} warn, {} fail",
        report.tool_version,
        report.contract,
        report.findings.len(),
        verdict.passed,
        verdict.warning.len(),
        verdict.failing.len()
    );
    let status = verdict.conclusion.to_finding_status();
    let evidence = if stale {
        vec![format!(
            "driftwatch {} reported at {}; source revision older than current {}",
            report.tool,
            report
                .source_revision
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string()),
            crate::core::manifest::CANONICAL_MANIFEST
        )]
    } else {
        vec![format!(
            "driftwatch {} reported {} finding(s) ({} pass, {} warn, {} fail)",
            report.tool,
            report.findings.len(),
            verdict.passed,
            verdict.warning.len(),
            verdict.failing.len()
        )]
    };
    Rollup {
        status,
        evidence,
        detail,
    }
}

#[cfg(test)]
mod policy_verdict_tests {
    use super::*;
    use crate::policy::PolicyFinding;

    fn finding(id: &str, severity: PolicySeverity, applicable: bool) -> PolicyFinding {
        PolicyFinding {
            id: id.to_string(),
            category: "documentation".to_string(),
            severity,
            applicable,
            message: format!("{id} reported a finding"),
            evidence: vec![],
            reason: None,
        }
    }

    fn report(findings: Vec<PolicyFinding>) -> PolicyReport {
        PolicyReport {
            tool: "driftwatch".to_string(),
            tool_version: "0.9.9".to_string(),
            contract: "0.1.0".to_string(),
            source_revision: None,
            findings,
        }
    }

    #[test]
    fn a_failing_applicable_rule_is_the_failing_conclusion() {
        let report = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Fail, true)]);
        let verdict = PolicyVerdict::judge(&report, false);
        assert_eq!(verdict.conclusion, PolicyConclusion::Failing);
        assert_eq!(verdict.failing, vec!["POLICY-DOCS-001".to_string()]);
        assert_eq!(verdict.warning, Vec::<String>::new());
        assert_eq!(verdict.applicable, 1);
        assert_eq!(verdict.passed, 0);
        assert!(!verdict.stale);
    }

    #[test]
    fn a_report_from_an_older_source_state_is_the_stale_conclusion() {
        let report = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Pass, true)]);
        let verdict = PolicyVerdict::judge(&report, true);
        assert_eq!(verdict.conclusion, PolicyConclusion::Stale);
        assert!(verdict.stale);
        assert_eq!(verdict.applicable, 1);
        assert_eq!(verdict.passed, 1);
    }

    #[test]
    fn a_report_no_rule_applied_to_is_the_unjudged_conclusion() {
        let report = report(vec![finding(
            "POLICY-DOCS-001",
            PolicySeverity::Pass,
            false,
        )]);
        let verdict = PolicyVerdict::judge(&report, false);
        assert_eq!(verdict.conclusion, PolicyConclusion::Unjudged);
        assert_eq!(verdict.applicable, 0);
        assert_eq!(verdict.passed, 0);
        assert_eq!(verdict.failing, Vec::<String>::new());
        assert_eq!(verdict.warning, Vec::<String>::new());
    }

    #[test]
    fn an_applicable_rule_that_warned_is_the_warning_conclusion() {
        let report = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Warn, true)]);
        let verdict = PolicyVerdict::judge(&report, false);
        assert_eq!(verdict.conclusion, PolicyConclusion::Warning);
        assert_eq!(verdict.warning, vec!["POLICY-DOCS-001".to_string()]);
        assert_eq!(verdict.applicable, 1);
        assert_eq!(verdict.passed, 0);
    }

    #[test]
    fn every_applicable_rule_passing_is_the_passed_conclusion() {
        let report = report(vec![
            finding("POLICY-DOCS-001", PolicySeverity::Pass, true),
            finding("POLICY-SEC-002", PolicySeverity::Pass, true),
        ]);
        let verdict = PolicyVerdict::judge(&report, false);
        assert_eq!(verdict.conclusion, PolicyConclusion::Passed);
        assert_eq!(verdict.applicable, 2);
        assert_eq!(verdict.passed, 2);
        assert_eq!(verdict.failing, Vec::<String>::new());
        assert_eq!(verdict.warning, Vec::<String>::new());
    }

    #[test]
    fn a_failing_rule_outranks_a_stale_report() {
        let report = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Fail, true)]);
        let verdict = PolicyVerdict::judge(&report, true);
        assert_eq!(verdict.conclusion, PolicyConclusion::Failing);
        assert!(verdict.stale);
    }

    #[test]
    fn a_stale_report_outranks_an_unjudged_report() {
        let report = report(vec![finding(
            "POLICY-DOCS-001",
            PolicySeverity::Pass,
            false,
        )]);
        let verdict = PolicyVerdict::judge(&report, true);
        assert_eq!(verdict.conclusion, PolicyConclusion::Stale);
        assert_eq!(verdict.applicable, 0);
    }

    #[test]
    fn a_warning_outranks_a_pass() {
        let report = report(vec![
            finding("POLICY-DOCS-001", PolicySeverity::Pass, true),
            finding("POLICY-SEC-002", PolicySeverity::Warn, true),
        ]);
        let verdict = PolicyVerdict::judge(&report, false);
        assert_eq!(verdict.conclusion, PolicyConclusion::Warning);
        assert_eq!(verdict.passed, 1);
        assert_eq!(verdict.warning, vec!["POLICY-SEC-002".to_string()]);
    }

    #[test]
    fn every_conclusion_maps_to_the_doctors_own_finding_status() {
        assert_eq!(
            PolicyConclusion::Failing.to_finding_status(),
            FindingStatus::Fail
        );
        assert_eq!(
            PolicyConclusion::Stale.to_finding_status(),
            FindingStatus::Warn
        );
        assert_eq!(
            PolicyConclusion::Unjudged.to_finding_status(),
            FindingStatus::Unavailable
        );
        assert_eq!(
            PolicyConclusion::Warning.to_finding_status(),
            FindingStatus::Warn
        );
        assert_eq!(
            PolicyConclusion::Passed.to_finding_status(),
            FindingStatus::Pass
        );
    }

    #[test]
    fn the_rollup_keeps_the_doctors_own_statuses() {
        let failing = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Fail, true)]);
        assert_eq!(
            aggregate_rollup(&failing, false).status,
            FindingStatus::Fail
        );
        let warned = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Warn, true)]);
        assert_eq!(aggregate_rollup(&warned, false).status, FindingStatus::Warn);
        let stale = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Pass, true)]);
        assert_eq!(aggregate_rollup(&stale, true).status, FindingStatus::Warn);
        // A report that is both stale and unjudged reads as a warning,
        // exactly as the doctor's ordering produces.
        let unjudged = report(vec![finding(
            "POLICY-DOCS-001",
            PolicySeverity::Pass,
            false,
        )]);
        assert_eq!(
            aggregate_rollup(&unjudged, true).status,
            FindingStatus::Warn
        );
        assert_eq!(
            aggregate_rollup(&unjudged, false).status,
            FindingStatus::Unavailable
        );
        let passed = report(vec![finding("POLICY-DOCS-001", PolicySeverity::Pass, true)]);
        assert_eq!(aggregate_rollup(&passed, false).status, FindingStatus::Pass);
    }

    #[test]
    fn the_rollup_keeps_the_doctors_own_counts_and_wording() {
        let report = report(vec![
            finding("POLICY-DOCS-001", PolicySeverity::Pass, true),
            finding("POLICY-SEC-002", PolicySeverity::Warn, true),
            finding("POLICY-ARCH-003", PolicySeverity::Fail, true),
            finding("POLICY-DOCS-004", PolicySeverity::Pass, false),
        ]);
        let rollup = aggregate_rollup(&report, false);
        assert_eq!(rollup.status, FindingStatus::Fail);
        assert_eq!(
            rollup.detail,
            "driftwatch 0.9.9 (contract 0.1.0) reported 4 finding(s): 1 pass, 1 warn, 1 fail"
        );
        assert_eq!(
            rollup.evidence,
            vec![
                "driftwatch driftwatch reported 4 finding(s) (1 pass, 1 warn, 1 fail)".to_string()
            ]
        );
    }
}
