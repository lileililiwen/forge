//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog::record::{CatalogRecord, SourceKind};

use super::contract::GAPS_CONTRACT_VERSION;
use super::model::{GapCategory, GapFilters, GapFinding, GapReport, GapStatus, RemediationClass};
use super::rules::evaluate;

/// Compute the gap findings for a single record. One finding per
/// category in the order [`GapCategory::all`] returns, except
/// `not_applicable` findings are still emitted so a consumer can count
/// the inapplicable controls explicitly.
pub fn findings_for_record(record: &CatalogRecord) -> Vec<GapFinding> {
    let mut out = Vec::with_capacity(GapCategory::all().len());
    for category in GapCategory::all() {
        out.push(evaluate(record, category));
    }
    out
}

/// Build the full gap report over the given records. When `project_id`
/// is `Some`, the report is single-project scoped and emits only
/// records for that id (the caller should pre-filter); the field stays
/// in the report envelope so a consumer can scope its queries.
///
/// Unavailable sources (those that returned a `SourceStatus` with
/// `state == "unavailable"`) are surfaced as one `unavailable` finding
/// per closed category, named by the source, so a failed read is never
/// reported as a silent zero. The `github` source is a permanent
/// "not implemented" state and is surfaced through the source status
/// only — it never generates findings.
pub fn build_report(
    records: &[CatalogRecord],
    source_statuses: &[crate::catalog::SourceStatus],
    project_id: Option<&str>,
) -> GapReport {
    let mut findings: Vec<GapFinding> = records.iter().flat_map(findings_for_record).collect();
    for status in source_statuses {
        if status.state == "unavailable" && status.source_kind != SourceKind::Github {
            for category in GapCategory::all() {
                findings.push(synthesize_unavailable_finding(status, category));
            }
        }
    }
    findings.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.category.id().cmp(b.category.id()))
            .then_with(|| a.subject.cmp(&b.subject))
    });
    GapReport {
        contract: GAPS_CONTRACT_VERSION.to_string(),
        project_id: project_id.map(|s| s.to_string()),
        findings,
    }
}

/// One synthesized `unavailable` finding for a source that could not be
/// read. The source string is used as the `project_id` surrogate (it is
/// the best identifier we have when no records were returned) so the
/// finding has a stable, unique id of the form
/// `gaps.<category>.<source>.<subject>`.
fn synthesize_unavailable_finding(
    status: &crate::catalog::SourceStatus,
    category: GapCategory,
) -> GapFinding {
    let reason = status
        .reason
        .clone()
        .unwrap_or_else(|| "source is unavailable".to_string());
    let subject = category.id();
    let id = format!("gaps.{}.{}.{}", category.id(), status.source, subject);
    let detail = format!("source is unavailable: {reason}");
    let evidence = vec![reason.clone()];
    GapFinding {
        id,
        status: GapStatus::Unavailable,
        category,
        subject: subject.to_string(),
        remediation_class: RemediationClass::Manual,
        project_id: status.source.clone(),
        source: status.source.clone(),
        source_kind: status.source_kind,
        source_revision: None,
        observed_at: String::new(),
        freshness: crate::catalog::record::Freshness::Unknown,
        evidence,
        detail,
    }
}

/// Render the gap report as a human-facing table. The table is not the
/// machine contract (JSON and NDJSON are); the layout exists so a
/// operator can scan findings without parsing JSON.
pub fn render_report_human(
    report: &GapReport,
    filtered: &[&GapFinding],
    sources: &[crate::catalog::SourceStatus],
    total: usize,
    filters: &GapFilters,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "forge project gaps — contract {}", report.contract);
    if let Some(id) = report.project_id.as_deref() {
        let _ = writeln!(out, "project: {id}");
    } else {
        let _ = writeln!(out, "project: (catalog)");
    }
    if sources.is_empty() {
        let _ = writeln!(out, "sources: (none)");
    } else {
        for status in sources {
            let _ = writeln!(
                out,
                "source: {} [{}] state={} records={}{}",
                status.source,
                status.source_kind,
                status.state,
                status.records,
                status
                    .reason
                    .as_deref()
                    .map(|r| format!(" reason={r}"))
                    .unwrap_or_default()
            );
        }
    }
    let _ = writeln!(
        out,
        "findings: {} of {} ({} categories, {} statuses, {} remediation-classes)",
        filtered.len(),
        total,
        filters.categories.len(),
        filters.statuses.len(),
        filters.remediation_classes.len(),
    );
    if filtered.is_empty() {
        let _ = writeln!(out, "(no findings)");
        return out;
    }
    #[allow(clippy::write_literal)]
    let _ = writeln!(
        out,
        "{:<44} {:<18} {:<14} {:<11} {:<10} {}",
        "ID", "CATEGORY", "SUBJECT", "STATUS", "REMED.", "DETAIL",
    );
    for finding in filtered {
        let _ = writeln!(
            out,
            "{:<44} {:<18} {:<14} {:<11} {:<10} {}",
            truncate(&finding.id, 44),
            finding.category.id(),
            truncate(&finding.subject, 14),
            finding.status.id(),
            finding.remediation_class.id(),
            truncate(&finding.detail, 80),
        );
    }
    out
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::record::{EvidenceState, Freshness, SourceKind};
    use crate::doctor::Remediation;

    #[allow(clippy::too_many_arguments)]
    fn record(
        project_id: &str,
        source: &str,
        kind: SourceKind,
        evidence: EvidenceState,
        name: Option<&str>,
        tags: &[&str],
        ci: Option<&str>,
        compose: Option<&str>,
        repository: Option<&str>,
    ) -> CatalogRecord {
        CatalogRecord {
            project_id: project_id.to_string(),
            name: name.map(|n| n.to_string()),
            source: source.to_string(),
            source_kind: kind,
            source_revision: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            observed_at: "2026-09-29T00:00:00Z".to_string(),
            freshness: Freshness::Current,
            profile: Some("rust-web".to_string()),
            lifecycle: Some("operational".to_string()),
            repository: repository.map(|r| r.to_string()),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            languages: vec!["rust".to_string()],
            ci: ci.map(|c| c.to_string()),
            compose: compose.map(|c| c.to_string()),
            evidence,
        }
    }

    #[test]
    fn vocabularies_parse_their_known_values() {
        for category in GapCategory::all() {
            assert_eq!(GapCategory::parse(category.id()), Some(category));
        }
        assert_eq!(GapCategory::parse("nope"), None);
        for status in [
            GapStatus::Pass,
            GapStatus::Warn,
            GapStatus::Fail,
            GapStatus::Unavailable,
            GapStatus::NotApplicable,
        ] {
            assert_eq!(GapStatus::parse(status.id()), Some(status));
        }
        assert_eq!(GapStatus::parse("bogus"), None);
        for class in [
            RemediationClass::Automatic,
            RemediationClass::Semantic,
            RemediationClass::Manual,
        ] {
            assert_eq!(RemediationClass::parse(class.id()), Some(class));
        }
        assert_eq!(RemediationClass::parse("nope"), None);
    }

    #[test]
    fn stable_id_includes_category_project_and_subject() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(description.id, "gaps.description.alpha.name");
        assert_eq!(description.status, GapStatus::Pass);
    }

    #[test]
    fn missing_name_in_local_is_a_distinct_finding() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            None,
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        assert_eq!(description.status, GapStatus::Fail);
        assert_eq!(tags.status, GapStatus::Pass);
    }

    #[test]
    fn empty_tags_yield_a_finding_separate_from_description() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &[],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(tags.status, GapStatus::Fail);
        assert_eq!(description.status, GapStatus::Pass);
        assert_ne!(tags.id, description.id);
    }

    #[test]
    fn unavailable_evidence_is_unavailable_not_fail() {
        let rec = record(
            "alpha",
            "workspace-registry",
            SourceKind::WorkspaceRegistry,
            EvidenceState::Unavailable,
            None,
            &[],
            None,
            None,
            None,
        );
        let findings = findings_for_record(&rec);
        for category in [
            GapCategory::Tags,
            GapCategory::Ci,
            GapCategory::Compose,
            GapCategory::Repository,
        ] {
            let f = findings.iter().find(|f| f.category == category).unwrap();
            assert_eq!(f.status, GapStatus::Unavailable, "{category:?}");
        }
    }

    #[test]
    fn stale_observation_warns_when_value_present() {
        let mut rec = record(
            "alpha",
            "inventory",
            SourceKind::Inventory,
            EvidenceState::Stale,
            None,
            &["platform"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        rec.freshness = crate::catalog::record::Freshness::Stale;
        let findings = findings_for_record(&rec);
        let ci = findings
            .iter()
            .find(|f| f.category == GapCategory::Ci)
            .unwrap();
        let compose = findings
            .iter()
            .find(|f| f.category == GapCategory::Compose)
            .unwrap();
        assert_eq!(ci.status, GapStatus::Warn);
        assert_eq!(compose.status, GapStatus::Warn);
    }

    #[test]
    fn github_source_marks_description_not_applicable() {
        let rec = record(
            "alpha",
            "github",
            SourceKind::Github,
            EvidenceState::Present,
            None,
            &["platform"],
            None,
            None,
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert_eq!(description.status, GapStatus::NotApplicable);
    }

    #[test]
    fn docs_is_not_applicable_for_every_source() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let docs = findings
            .iter()
            .find(|f| f.category == GapCategory::Docs)
            .unwrap();
        assert_eq!(docs.status, GapStatus::NotApplicable);
    }

    #[test]
    fn filters_compose_with_and_semantics() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &[],
            None,
            None,
            Some("https://example.invalid/alpha.git"),
        );
        let findings = findings_for_record(&rec);
        let f = GapFilters {
            categories: vec![GapCategory::Tags],
            statuses: vec![GapStatus::Fail],
            remediation_classes: vec![],
        };
        let tags = findings
            .iter()
            .find(|f| f.category == GapCategory::Tags)
            .unwrap();
        let description = findings
            .iter()
            .find(|f| f.category == GapCategory::Description)
            .unwrap();
        assert!(f.matches(tags));
        assert!(!f.matches(description));
    }

    #[test]
    fn report_orders_findings_by_project_then_source_then_category_then_subject() {
        let rec1 = record(
            "beta",
            "workspace-registry",
            SourceKind::WorkspaceRegistry,
            EvidenceState::Present,
            None,
            &["platform"],
            Some("passing"),
            None,
            Some("https://example.invalid/beta.git"),
        );
        let rec2 = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let report = build_report(&[rec1, rec2], &[], None);
        let mut last: Option<(&str, &str, &str, &str)> = None;
        for f in &report.findings {
            let key = (
                f.project_id.as_str(),
                f.source.as_str(),
                f.category.id(),
                f.subject.as_str(),
            );
            if let Some(prev) = last.as_ref() {
                assert!(prev <= &key, "{prev:?} > {key:?}");
            }
            last = Some(key);
        }
    }

    #[test]
    fn finding_evidence_is_redacted() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://user:ghp_abcdefghijklmnopqrstuvwxyz0123456789@x/y.git"),
        );
        let findings = findings_for_record(&rec);
        for finding in &findings {
            for line in &finding.evidence {
                assert!(!line.contains("ghp_abcdef"), "{line}");
            }
            assert!(!finding.detail.contains("ghp_abcdef"), "{}", finding.detail);
        }
    }

    #[test]
    fn remediation_class_maps_to_doctor_remediation() {
        assert_eq!(
            RemediationClass::Automatic.as_doctor(),
            Remediation::Automatic
        );
        assert_eq!(RemediationClass::Semantic.as_doctor(), Remediation::Ai);
        assert_eq!(RemediationClass::Manual.as_doctor(), Remediation::Manual);
    }

    #[test]
    fn clean_report_has_no_fail_or_unavailable() {
        let rec = record(
            "alpha",
            "local",
            SourceKind::Local,
            EvidenceState::Present,
            Some("Alpha"),
            &["product"],
            Some("passing"),
            Some("compose_ready"),
            Some("https://example.invalid/alpha.git"),
        );
        let report = build_report(std::slice::from_ref(&rec), &[], None);
        assert!(report.clean());
    }
}
