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

    let (build_present, build_evidence) = descriptor
        .map(|_| has_build_definition(dir, profile_id))
        .unwrap_or((false, vec!["unknown profile".to_string()]));
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
            "project structure matches the profile layout (L1 structure)",
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
            at_least(Maturity::L3),
            registered,
            vec![if registered {
                "project is registered".to_string()
            } else {
                "project is not registered".to_string()
            }],
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
    let mut applicable = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;
    let mut pass = 0usize;
    for pf in &report.findings {
        if !pf.applicable {
            continue;
        }
        applicable += 1;
        match pf.severity {
            PolicySeverity::Pass => pass += 1,
            PolicySeverity::Warn => warn += 1,
            PolicySeverity::Fail => fail += 1,
        }
    }
    let detail = format!(
        "driftwatch {} (contract {}) reported {} finding(s): {} pass, {} warn, {} fail",
        report.tool_version,
        report.contract,
        report.findings.len(),
        pass,
        warn,
        fail
    );
    let status = if fail > 0 {
        FindingStatus::Fail
    } else if warn > 0 || stale {
        FindingStatus::Warn
    } else if applicable == 0 {
        FindingStatus::Unavailable
    } else {
        FindingStatus::Pass
    };
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
            pass,
            warn,
            fail
        )]
    };
    Rollup {
        status,
        evidence,
        detail,
    }
}
