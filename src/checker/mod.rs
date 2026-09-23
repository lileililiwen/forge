//! Forge as an external DriftWatch checker.
//!
//! This module projects the existing read-only assessment planes (doctor
//! findings, the optional DriftWatch policy plane carried through doctor,
//! the selected governance observation and the profile readiness state)
//! into the Driftwatchdog external-checker protocol: one JSON document on
//! stdout carrying an `alerts` array whose severities stay inside the
//! protocol vocabulary. It is a projection only — it never runs a second
//! assessment, never journals, and never mutates the assessed project.
//!
//! Protocol contract mirrored from the sibling `config-checker-protocol`
//! spec (`driftwatchdog`): `alerts` must always be present (a document
//! without the key is a protocol error, never success); unknown top-level
//! fields are ignored and unknown per-alert fields are tolerated; each
//! alert requires non-empty `severity`, `message`, `source` and `symbol`;
//! the parser caps at 10_000 alerts and 64 KiB per message. Severity
//! classification treats `error` as Error, `info` as Info and everything
//! else as Warning, so this emitter only ever writes `error` and
//! `warning`.

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::path::Path;

use crate::core::ForgeError;
use crate::doctor::{DoctorReport, FindingStatus};
use crate::governance::{GovernanceObservation, ProviderStatus};
use crate::policy::redact_credentials;
use crate::profile::{inspect_profile, ProfileSupportStatus};
use crate::readiness::ReadinessStatus;

/// Versioned contract for the checker emission surface.
pub const CHECKER_CONTRACT_VERSION: &str = "0.1.0";

/// Protocol discriminator a tolerant checker parser may ignore.
pub const CHECKER_SCHEMA_ID: &str = "forge-checker/0.1.0";

/// Default bound on the number of alerts in one document.
pub const DEFAULT_MAX_ALERTS: usize = 64;

/// Hard bound inherited from the sibling protocol parser: a document with
/// more alerts would be rejected outright, so it must never be emitted.
pub const PROTOCOL_MAX_ALERTS: usize = 10_000;

/// Per-alert message bound in characters. The sibling parser caps raw
/// messages at 64 KiB bytes; staying far below that keeps gate output
/// readable and truncation explicit.
pub const MAX_ALERT_MESSAGE_CHARS: usize = 480;

/// Severity vocabulary of the checker protocol. Passing and
/// not-applicable evidence is omitted, so only these two appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Error,
    Warning,
}

/// One normalized checker alert. Field order is the protocol boundary:
/// `severity`, `message`, `source`, `symbol` are all required non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckerAlert {
    pub severity: AlertSeverity,
    pub message: String,
    pub source: String,
    pub symbol: String,
}

/// The machine document printed on stdout. `alerts` is a plain array so
/// it is always serialized, even when empty — an absent key would be a
/// protocol error at the consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckerDocument {
    pub schema: String,
    pub generated_at: String,
    pub alerts: Vec<CheckerAlert>,
}

/// Current RFC 3339 timestamp for document generation.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Project doctor findings into alerts: `fail` becomes `error`,
/// `warn` becomes `warning`, `unavailable` becomes a `warning` naming
/// the missing evidence, and passing or not-applicable findings are
/// omitted. Policy-derived findings re-enter through this same plane
/// only when the caller explicitly included the policy outcome.
pub fn doctor_alerts(dir: &Path, report: &DoctorReport) -> Vec<CheckerAlert> {
    report
        .findings
        .iter()
        .filter(|finding| finding.applicable)
        .filter_map(|finding| {
            let severity = match finding.status {
                FindingStatus::Pass => return None,
                FindingStatus::Fail => AlertSeverity::Error,
                FindingStatus::Warn | FindingStatus::Unavailable => AlertSeverity::Warning,
            };
            let mut parts: Vec<String> = Vec::new();
            parts.push(format!("{}: {}", finding.id, finding.detail));
            for line in &finding.evidence {
                parts.push(line.clone());
            }
            Some(CheckerAlert {
                severity,
                message: parts.join("; "),
                source: finding_source(dir, finding),
                symbol: if is_policy_rule_id(&finding.id) {
                    finding.id.clone()
                } else {
                    format!("doctor/{}", finding.id)
                },
            })
        })
        .collect()
}

/// True for findings that originate from the DriftWatch policy plane
/// (the `driftwatch-policy` rollup and per-rule `driftwatch-<RULE>`
/// ids), whose symbols stay the stable POLICY-ID form. The
/// `driftwatch-config` finding is doctor's own presence detection, not
/// a policy rule, so it carries the `doctor/` prefix like any finding.
fn is_policy_rule_id(id: &str) -> bool {
    id.starts_with("driftwatch-") && id != "driftwatch-config"
}

/// Source of a doctor finding: a project-relative path when the evidence
/// cites a real project file, otherwise the `doctor` plane name.
fn finding_source(dir: &Path, finding: &crate::doctor::Finding) -> String {
    for line in &finding.evidence {
        for token in line.split_whitespace() {
            let token = token.trim_matches(|c: char| {
                matches!(
                    c,
                    '\'' | '"' | '`' | ',' | ';' | ':' | '(' | ')' | '[' | ']'
                )
            });
            if is_project_relative_file(dir, token) {
                return token.to_string();
            }
        }
    }
    "doctor".to_string()
}

/// True only for clean relative paths inside the project: no leading
/// `/`, no `~`, no `..` traversal, no drive/URL-ish prefixes, and the
/// path must actually exist under `dir`. Anything else falls back to a
/// plane name so the document can never leak host layout.
fn is_project_relative_file(dir: &Path, token: &str) -> bool {
    if token.is_empty() || token.len() > 256 {
        return false;
    }
    if !token.contains('.') && !token.contains('/') {
        return false;
    }
    if token.starts_with('/') || token.starts_with('~') || token.contains('\\') {
        return false;
    }
    if token.contains("://") {
        return false;
    }
    if token.split('/').any(|segment| segment == "..") {
        return false;
    }
    let candidate = dir.join(token);
    let base = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    match candidate.canonicalize() {
        Ok(resolved) => resolved.starts_with(&base),
        Err(_) => false,
    }
}

/// Project the selected governance observation into alerts. `fail` and
/// `blocked` become `error`; `unknown`, `unavailable`, `stale` and
/// `incompatible` become `warning` naming the missing evidence;
/// `pass` and `disabled` (a deliberate operator choice) are omitted.
/// A governance plane that could not be evaluated at all surfaces as a
/// warning rather than dropping the plane silently.
pub fn governance_alerts(result: &Result<GovernanceObservation, ForgeError>) -> Vec<CheckerAlert> {
    let observation = match result {
        Ok(observation) => observation,
        Err(err) => {
            return vec![CheckerAlert {
                severity: AlertSeverity::Warning,
                message: format!("governance plane unavailable: {err}"),
                source: "governance".to_string(),
                symbol: "governance/unavailable".to_string(),
            }]
        }
    };
    let severity = match observation.status {
        ProviderStatus::Pass | ProviderStatus::Disabled => return Vec::new(),
        ProviderStatus::Fail | ProviderStatus::Blocked => AlertSeverity::Error,
        ProviderStatus::Unknown
        | ProviderStatus::Unavailable
        | ProviderStatus::Stale
        | ProviderStatus::Incompatible => AlertSeverity::Warning,
    };
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!(
        "governance provider `{}` status `{}`",
        observation.provider,
        status_label(observation.status)
    ));
    if let Some(detail) = observation.detail.as_deref() {
        parts.push(detail.to_string());
    }
    for line in &observation.evidence {
        parts.push(line.clone());
    }
    vec![CheckerAlert {
        severity,
        message: parts.join("; "),
        source: "governance".to_string(),
        symbol: format!("governance/{}", observation.provider),
    }]
}

/// Human-readable kebab-case status label matching the governance wire
/// vocabulary.
fn status_label(status: ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Pass => "pass",
        ProviderStatus::Fail => "fail",
        ProviderStatus::Blocked => "blocked",
        ProviderStatus::Unknown => "unknown",
        ProviderStatus::Unavailable => "unavailable",
        ProviderStatus::Stale => "stale",
        ProviderStatus::Disabled => "disabled",
        ProviderStatus::Incompatible => "incompatible",
    }
}

/// Project the profile readiness state into alerts. A supported profile
/// carries certified native-template evidence from the readiness surface,
/// so it contributes nothing; a planned catalog candidate is reported as
/// `unverified` evidence and must surface as a warning naming the plane,
/// never dropped. An unknown profile contributes no readiness alert —
/// doctor already reports it as a fail finding, and the checker must not
/// double-count one gap.
pub fn readiness_alerts(profile: Option<&str>) -> Vec<CheckerAlert> {
    let Some(profile) = profile else {
        return Vec::new();
    };
    let descriptor = match inspect_profile(profile) {
        Ok(descriptor) => descriptor,
        Err(_) => return Vec::new(),
    };
    if descriptor.support_status == ProfileSupportStatus::Supported {
        return Vec::new();
    }
    vec![CheckerAlert {
        severity: AlertSeverity::Warning,
        message: format!(
            "readiness {}: profile '{profile}' is a planned catalog candidate with no \
             certified native template evidence; no native build or test run was recorded",
            ReadinessStatus::Unverified.id()
        ),
        source: "readiness".to_string(),
        symbol: format!("readiness/{profile}"),
    }]
}

/// Assemble the final document: plane order is doctor, governance,
/// readiness; every field is sanitized (credential redaction, project
/// path removal, message bound) and the list is truncated to the bound
/// with an explicit summary alert naming the dropped count.
pub fn build_document(
    dir: &Path,
    doctor: &DoctorReport,
    governance: &Result<GovernanceObservation, ForgeError>,
    generated_at: &str,
    max_alerts: usize,
) -> CheckerDocument {
    let mut alerts = doctor_alerts(dir, doctor);
    alerts.extend(governance_alerts(governance));
    alerts.extend(readiness_alerts(doctor.profile.as_deref()));
    let alerts: Vec<CheckerAlert> = alerts
        .into_iter()
        .map(|alert| sanitize_alert(dir, alert))
        .collect();
    let alerts = truncate_alerts(alerts, max_alerts);
    CheckerDocument {
        schema: CHECKER_SCHEMA_ID.to_string(),
        generated_at: generated_at.to_string(),
        alerts,
    }
}

/// Enforce the alert-count bound. When the mapping exceeds the bound the
/// document keeps the first `max - 1` alerts and ends with one summary
/// alert naming how many were dropped, so truncation is observable and
/// never silently hides a `fail`.
pub fn truncate_alerts(alerts: Vec<CheckerAlert>, max_alerts: usize) -> Vec<CheckerAlert> {
    let max_alerts = max_alerts.clamp(1, PROTOCOL_MAX_ALERTS);
    if alerts.len() <= max_alerts {
        return alerts;
    }
    let keep = max_alerts - 1;
    let dropped = alerts.len() - keep;
    let mut out = alerts.into_iter().take(keep).collect::<Vec<_>>();
    out.push(CheckerAlert {
        severity: AlertSeverity::Warning,
        message: format!("check output truncated: {dropped} additional alerts dropped"),
        source: "doctor".to_string(),
        symbol: "check/truncated".to_string(),
    });
    out
}

/// Defense in depth on every emitted field: credential-shaped substrings
/// are redacted through the shared policy redactor and the assessed
/// project's absolute layout is replaced by `<project>`, so a checker
/// document never leaks host paths or secrets.
fn sanitize_alert(dir: &Path, alert: CheckerAlert) -> CheckerAlert {
    CheckerAlert {
        severity: alert.severity,
        message: sanitize_text(dir, &alert.message),
        source: sanitize_text(dir, &alert.source),
        symbol: sanitize_text(dir, &alert.symbol),
    }
}

fn sanitize_text(dir: &Path, text: &str) -> String {
    let mut out = text.to_string();
    let display = dir.display().to_string();
    if !display.is_empty() && out.contains(&display) {
        out = out.replace(&display, "<project>");
    }
    if let Ok(canonical) = dir.canonicalize() {
        let canonical = canonical.display().to_string();
        if !canonical.is_empty() && out.contains(&canonical) {
            out = out.replace(&canonical, "<project>");
        }
    }
    let out = redact_credentials(&out);
    bound_message(&out)
}

/// Bound one message to [`MAX_ALERT_MESSAGE_CHARS`] characters at a
/// char boundary with an explicit truncation marker.
fn bound_message(text: &str) -> String {
    if text.chars().count() <= MAX_ALERT_MESSAGE_CHARS {
        return text.to_string();
    }
    let kept: String = text.chars().take(MAX_ALERT_MESSAGE_CHARS - 3).collect();
    format!("{kept}...")
}

/// Render the document as the exact bytes printed on stdout: one compact
/// JSON line. Both CLI formats emit these same bytes so `--format json`
/// cannot alter checker output.
pub fn render_document(document: &CheckerDocument) -> Result<String, ForgeError> {
    serde_json::to_string(document).map_err(|err| ForgeError::CheckInvalid {
        reason: format!("cannot encode checker document: {err}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::{Finding, Remediation};
    use crate::governance::GovernanceObservation;
    use std::fs;
    use tempfile::TempDir;

    fn finding(id: &str, status: FindingStatus, evidence: Vec<&str>) -> Finding {
        Finding {
            id: id.to_string(),
            status,
            evidence: evidence.into_iter().map(String::from).collect(),
            applicable: true,
            remediation: Remediation::Manual,
            detail: format!("{id} detail"),
        }
    }

    fn report_with(findings: Vec<Finding>) -> DoctorReport {
        DoctorReport {
            path: ".".to_string(),
            profile: Some("rust-web".to_string()),
            policy_version: "0.1.0".to_string(),
            current_maturity: Some("L1".to_string()),
            target_maturity: Some("L1".to_string()),
            findings,
            controls: Vec::new(),
            healthy: false,
            stale: false,
        }
    }

    fn gov(status: ProviderStatus) -> Result<GovernanceObservation, ForgeError> {
        Ok(GovernanceObservation {
            provider: "local".to_string(),
            protocol_version: "0.1.0".to_string(),
            project_id: "proj".to_string(),
            project_path: "/host/layout/proj".to_string(),
            status,
            observed_at: "2026-09-24T00:00:00Z".to_string(),
            source_revision: None,
            evidence: vec![],
            detail: Some("provider detail".to_string()),
            metadata: serde_json::Value::Null,
        })
    }

    #[test]
    fn severity_vocabulary_covers_protocol_only_error_and_warning() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![
            finding(
                "manifest-valid",
                FindingStatus::Pass,
                vec!["forge.yaml parses"],
            ),
            finding(
                "profile-known",
                FindingStatus::Fail,
                vec!["profile 'bogus' matches no MVP profile descriptor"],
            ),
            finding(
                "dependency-drift",
                FindingStatus::Warn,
                vec!["dependency evidence disagrees"],
            ),
            finding(
                "repository",
                FindingStatus::Unavailable,
                vec!["not a git repository; remote cannot be determined"],
            ),
        ]);
        let alerts = doctor_alerts(tmp.path(), &report);
        assert_eq!(alerts.len(), 3, "pass findings are omitted");
        assert_eq!(alerts[0].severity, AlertSeverity::Error);
        assert_eq!(alerts[0].symbol, "doctor/profile-known");
        assert_eq!(alerts[1].severity, AlertSeverity::Warning);
        assert_eq!(alerts[2].severity, AlertSeverity::Warning);
        assert!(alerts[2].message.contains("not a git repository"));
    }

    #[test]
    fn not_applicable_findings_are_omitted() {
        let tmp = TempDir::new().unwrap();
        let mut skip = finding("ci-config", FindingStatus::Warn, vec!["no ci"]);
        skip.applicable = false;
        let alerts = doctor_alerts(tmp.path(), &report_with(vec![skip]));
        assert!(alerts.is_empty());
    }

    #[test]
    fn policy_ids_keep_their_stable_rule_symbol() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![
            finding(
                "driftwatch-AUTH-001",
                FindingStatus::Fail,
                vec!["auth marker missing"],
            ),
            finding(
                "driftwatch-policy",
                FindingStatus::Unavailable,
                vec!["adapter did not produce a report"],
            ),
            finding(
                "driftwatch-config",
                FindingStatus::Warn,
                vec!["no driftwatch configuration"],
            ),
        ]);
        let alerts = doctor_alerts(tmp.path(), &report);
        assert_eq!(alerts[0].symbol, "driftwatch-AUTH-001");
        assert_eq!(alerts[1].symbol, "driftwatch-policy");
        assert_eq!(alerts[2].symbol, "doctor/driftwatch-config");
        assert_eq!(alerts[1].severity, AlertSeverity::Warning);
    }

    #[test]
    fn file_scoped_findings_use_project_relative_source() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        let report = report_with(vec![finding(
            "dependency-drift",
            FindingStatus::Warn,
            vec!["Cargo.toml disagrees with the manifest"],
        )]);
        let alerts = doctor_alerts(tmp.path(), &report);
        assert_eq!(alerts[0].source, "Cargo.toml");
    }

    #[test]
    fn absolute_and_traversal_tokens_never_become_source() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![
            finding("x", FindingStatus::Warn, vec!["/etc/passwd was inspected"]),
            finding(
                "y",
                FindingStatus::Warn,
                vec!["outside via ../secrets/file.txt"],
            ),
            finding(
                "z",
                FindingStatus::Warn,
                vec!["remote https://example.test/a/b.git seen"],
            ),
        ]);
        let alerts = doctor_alerts(tmp.path(), &report);
        for alert in &alerts {
            assert_eq!(alert.source, "doctor", "{:?}", alert.source);
        }
    }

    #[test]
    fn governance_status_mapping() {
        let ok = governance_alerts(&gov(ProviderStatus::Pass));
        assert!(ok.is_empty(), "pass must not alert");
        let off = governance_alerts(&gov(ProviderStatus::Disabled));
        assert!(
            off.is_empty(),
            "disabled is an operator choice, not missing evidence"
        );
        let fail = governance_alerts(&gov(ProviderStatus::Fail));
        assert_eq!(fail[0].severity, AlertSeverity::Error);
        assert_eq!(fail[0].source, "governance");
        assert_eq!(fail[0].symbol, "governance/local");
        for status in [
            ProviderStatus::Unknown,
            ProviderStatus::Unavailable,
            ProviderStatus::Stale,
            ProviderStatus::Incompatible,
        ] {
            let alerts = governance_alerts(&gov(status));
            assert_eq!(alerts[0].severity, AlertSeverity::Warning, "{status:?}");
        }
    }

    #[test]
    fn governance_evaluation_error_degrades_to_warning() {
        let err: Result<GovernanceObservation, ForgeError> = Err(ForgeError::GovernanceInvalid {
            reason: "providers.yaml is not readable".to_string(),
        });
        let alerts = governance_alerts(&err);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
        assert_eq!(alerts[0].symbol, "governance/unavailable");
        assert!(alerts[0].message.contains("providers.yaml"));
    }

    #[test]
    fn readiness_planned_profile_is_unverified_warning() {
        let alerts = readiness_alerts(Some("python-ai"));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
        assert_eq!(alerts[0].source, "readiness");
        assert_eq!(alerts[0].symbol, "readiness/python-ai");
        assert!(alerts[0].message.contains("unverified"));
        assert!(alerts[0].message.contains("native"));
    }

    #[test]
    fn readiness_supported_and_unknown_profiles_emit_nothing() {
        assert!(readiness_alerts(Some("rust-web")).is_empty());
        assert!(readiness_alerts(Some("totally-unknown")).is_empty());
        assert!(readiness_alerts(None).is_empty());
    }

    #[test]
    fn truncation_keeps_bound_and_names_dropped_count() {
        let alerts: Vec<CheckerAlert> = (0..70)
            .map(|i| CheckerAlert {
                severity: AlertSeverity::Warning,
                message: format!("m{i}"),
                source: "doctor".to_string(),
                symbol: format!("doctor/m{i}"),
            })
            .collect();
        let out = truncate_alerts(alerts.clone(), 64);
        assert_eq!(out.len(), 64);
        assert_eq!(out[63].symbol, "check/truncated");
        assert!(
            out[63].message.contains("7 additional"),
            "{:?}",
            out[63].message
        );
        let exact = truncate_alerts(alerts.clone(), 70);
        assert_eq!(exact.len(), 70);
        assert_eq!(exact[69].symbol, "doctor/m69");
        let single = truncate_alerts(alerts, 1);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].symbol, "check/truncated");
    }

    #[test]
    fn messages_are_char_bounded_and_redacted() {
        let tmp = TempDir::new().unwrap();
        let long = "x".repeat(2_000);
        let report = report_with(vec![finding(
            "build-config",
            FindingStatus::Fail,
            vec![&long],
        )]);
        let doc = build_document(tmp.path(), &report, &gov(ProviderStatus::Pass), "t", 64);
        assert_eq!(doc.alerts.len(), 1);
        assert!(doc.alerts[0].message.chars().count() <= MAX_ALERT_MESSAGE_CHARS);
        assert!(doc.alerts[0].message.ends_with("..."));

        let leaky = report_with(vec![finding(
            "security-scan",
            FindingStatus::Fail,
            vec!["token ghp_abcdefghijklmnopqrstuvwxyz0123456789 present"],
        )]);
        let doc = build_document(tmp.path(), &leaky, &gov(ProviderStatus::Pass), "t", 64);
        assert!(doc.alerts[0].message.contains("[REDACTED]"));
        assert!(!doc.alerts[0].message.contains("ghp_abcdef"));
    }

    #[test]
    fn project_absolute_paths_never_survive_sanitization() {
        let tmp = TempDir::new().unwrap();
        let leaked = format!("checked {}", tmp.path().display());
        let report = report_with(vec![finding(
            "docs-present",
            FindingStatus::Warn,
            vec![&leaked],
        )]);
        let doc = build_document(tmp.path(), &report, &gov(ProviderStatus::Pass), "t", 64);
        let text = serde_json::to_string(&doc).unwrap();
        assert!(!text.contains(&tmp.path().display().to_string()), "{text}");
        assert!(text.contains("<project>"));
    }

    #[test]
    fn empty_document_still_carries_the_alerts_key() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![finding("manifest-valid", FindingStatus::Pass, vec![])]);
        let doc = build_document(tmp.path(), &report, &gov(ProviderStatus::Pass), "gen-1", 64);
        assert!(doc.alerts.is_empty());
        let text = render_document(&doc).unwrap();
        assert!(text.contains("\"alerts\":[]"), "{text}");
        assert!(text.contains("\"schema\":\"forge-checker/0.1.0\""));
        assert!(text.contains("\"generated_at\":\"gen-1\""));
    }

    #[test]
    fn document_is_deterministic_except_generated_at() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![
            finding("repository", FindingStatus::Warn, vec!["no origin remote"]),
            finding("driftwatch-LEAK-001", FindingStatus::Fail, vec!["secret"]),
        ]);
        let a = build_document(tmp.path(), &report, &gov(ProviderStatus::Stale), "t-1", 64);
        let b = build_document(tmp.path(), &report, &gov(ProviderStatus::Stale), "t-2", 64);
        assert_eq!(a.generated_at, "t-1");
        assert_eq!(b.generated_at, "t-2");
        assert_eq!(a.schema, b.schema);
        assert_eq!(a.alerts, b.alerts);
    }

    #[test]
    fn every_alert_carries_the_four_required_non_empty_fields() {
        let tmp = TempDir::new().unwrap();
        let report = report_with(vec![
            finding("features-compatible", FindingStatus::Fail, vec!["bad"]),
            finding("registry-observation", FindingStatus::Warn, vec!["stale"]),
        ]);
        let doc = build_document(
            tmp.path(),
            &report,
            &gov(ProviderStatus::Unavailable),
            "t",
            64,
        );
        assert!(doc.alerts.len() >= 3);
        for alert in &doc.alerts {
            assert!(!alert.message.is_empty());
            assert!(!alert.source.is_empty());
            assert!(!alert.symbol.is_empty());
            let serialized = serde_json::to_value(alert).unwrap();
            assert!(matches!(
                serialized["severity"].as_str().unwrap_or_default(),
                "error" | "warning"
            ));
        }
    }
}
