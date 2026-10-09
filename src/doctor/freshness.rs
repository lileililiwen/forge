//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::policy::PolicyReport;
use std::fs;
use std::path::Path;

use super::model::{Finding, FindingStatus, Remediation};
use super::probes::file_exists;

/// Convert the read-only docs freshness assessment into typed
/// findings. Each enabled locale gets a stable `docs-<locale>`
/// rule: `pass` when the recorded source hash matches, `warn`
/// when the derivative is stale, never translated or flagged
/// `needs-review`, and `fail` when the locale is misconfigured
/// (missing source, escaping paths). Disabled locales are
/// skipped entirely so automatic workflows never touch them.
/// A `docs-freshness` rollup carries the worst status; it is
/// present only when at least one locale is enabled.
pub(super) fn docs_freshness_findings(dir: &Path, manifest: &Manifest) -> Vec<Finding> {
    let assessments = match crate::docs::assess_freshness(dir, manifest) {
        Ok(a) => a,
        Err(err) => {
            return vec![Finding::new(
                "docs-freshness",
                FindingStatus::Fail,
                vec![format!("docs configuration is invalid: {err}")],
                true,
                Remediation::Manual,
                "docs configuration is invalid; fix `docs` in forge.yaml",
            )];
        }
    };
    if assessments.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut pass = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;
    for assessment in &assessments {
        let id = format!("docs-{}", assessment.locale);
        let mut evidence = vec![format!(
            "locale `{}` status: {}",
            assessment.locale,
            assessment.status_label()
        )];
        if let Some(hash) = &assessment.source_hash {
            evidence.push(format!("source_hash: {hash}"));
        }
        if let Some(derivative) = &assessment.derivative {
            evidence.push(format!("derivative: {derivative}"));
        }
        let (status, remediation, detail) = match &assessment.status {
            crate::docs::FreshnessStatus::Current { review } => {
                pass += 1;
                (
                    FindingStatus::Pass,
                    Remediation::Ai,
                    format!(
                        "derivative for locale `{}` is current (review: {})",
                        assessment.locale,
                        review.label()
                    ),
                )
            }
            crate::docs::FreshnessStatus::Stale => {
                warn += 1;
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "source changed since locale `{}` was translated; re-run `forge docs translate {}`",
                        assessment.locale, assessment.locale
                    ),
                )
            }
            crate::docs::FreshnessStatus::NeverTranslated => {
                warn += 1;
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "locale `{}` is enabled but has no derivative; run `forge docs translate {}`",
                        assessment.locale, assessment.locale
                    ),
                )
            }
            crate::docs::FreshnessStatus::NeedsReview { reasons } => {
                warn += 1;
                for reason in reasons {
                    evidence.push(format!("review-reason: {reason}"));
                }
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "derivative for locale `{}` needs review: {}",
                        assessment.locale,
                        if reasons.is_empty() {
                            "unspecified".to_string()
                        } else {
                            reasons.join("; ")
                        }
                    ),
                )
            }
            crate::docs::FreshnessStatus::Misconfigured { reason } => {
                fail += 1;
                (
                    FindingStatus::Fail,
                    Remediation::Manual,
                    format!("locale `{}` is misconfigured: {reason}", assessment.locale),
                )
            }
        };
        out.push(Finding::new(
            id.as_str(),
            status,
            evidence,
            true,
            remediation,
            detail,
        ));
    }
    let status = if fail > 0 {
        FindingStatus::Fail
    } else if warn > 0 {
        FindingStatus::Warn
    } else {
        FindingStatus::Pass
    };
    out.push(Finding::new(
        "docs-freshness",
        status,
        vec![format!(
            "translation locales: {} pass, {warn} warn, {fail} fail",
            pass
        )],
        true,
        Remediation::Ai,
        if fail > 0 {
            "one or more translation locales are misconfigured".to_string()
        } else if warn > 0 {
            "one or more derivatives are stale, missing or need review".to_string()
        } else {
            "all enabled translation locales are current".to_string()
        },
    ));
    out
}

pub(super) fn observation_is_stale_from_report(dir: &Path, report: &PolicyReport) -> bool {
    let Some(revision) = report.source_revision else {
        return false;
    };
    let current = manifest_mtime(dir);
    let Some(current) = current else {
        return false;
    };
    let now_secs = current
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let now = chrono::DateTime::<chrono::Utc>::from_timestamp(now_secs, 0);
    match now {
        Some(now) => now > revision,
        None => false,
    }
}

pub(super) fn marker_files_present(
    dir: &Path,
    names: &[&str],
    dir_markers: &[&str],
) -> (bool, Vec<String>) {
    let mut evidence = Vec::new();
    for name in names {
        if file_exists(dir, name) {
            evidence.push((*name).to_string());
        }
    }
    for name in dir_markers {
        if dir.join(name).is_dir() {
            evidence.push(format!("{name}/"));
        }
    }
    if evidence.is_empty() {
        (false, vec![format!("none of {} present", names.join(", "))])
    } else {
        (true, evidence)
    }
}

pub(super) fn manifest_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(dir.join("forge.yaml"))
        .and_then(|m| m.modified())
        .ok()
}

pub(super) fn observation_is_stale(
    observed_at: &str,
    mtime: Option<std::time::SystemTime>,
) -> bool {
    let mtime = match mtime {
        Some(t) => t,
        None => return false,
    };
    let observed = match chrono::DateTime::parse_from_rfc3339(observed_at) {
        Ok(dt) => dt,
        Err(_) => return false,
    };
    let mtime_chrono: chrono::DateTime<chrono::Utc> = mtime.into();
    mtime_chrono > observed.with_timezone(&chrono::Utc)
}
