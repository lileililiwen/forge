//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::portfolio::interest::{
    normalize_timestamp, InterestMetric, InterestSnapshot, MAX_METRIC_VALUE,
};
use chrono::{DateTime, Utc};

use super::readiness::{
    NotReadyReason, Readiness, ReadinessEvidence, ReadinessReason, ReadinessVerdict,
};

/// Bound an operator-declared threshold. Absence is legal (it becomes
/// the `threshold-not-declared` verdict); an out-of-range value is a
/// typed refusal, never a clamp.
pub fn validate_threshold(threshold: u64) -> Result<u64, String> {
    if threshold > MAX_METRIC_VALUE {
        return Err(format!(
            "min-value must be between 0 and {MAX_METRIC_VALUE}"
        ));
    }
    Ok(threshold)
}

/// Parse `<START>..<END>`, normalizing both sides to UTC and requiring
/// `end > start`.
pub fn parse_window(raw: &str) -> Result<(String, String), String> {
    let (start_raw, end_raw) = raw
        .split_once("..")
        .ok_or("window must be `<START>..<END>` with each side an RFC 3339 timestamp")?;
    let start = normalize_timestamp("window start", start_raw.trim())
        .map_err(|err| format!("window start is invalid: {err}"))?;
    let end = normalize_timestamp("window end", end_raw.trim())
        .map_err(|err| format!("window end is invalid: {err}"))?;
    if end <= start {
        return Err(format!(
            "window end `{end}` must be later than window start `{start}`"
        ));
    }
    Ok((start, end))
}

fn threshold_not_declared() -> ReadinessReason {
    ReadinessReason {
        reason: NotReadyReason::ThresholdNotDeclared,
        detail: "no threshold was declared; readiness requires an operator-declared threshold"
            .to_string(),
    }
}

fn stale_reason(
    snapshot: &InterestSnapshot,
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> ReadinessReason {
    // Whole days since the window end, matching the freshness
    // derivation: an unreadable bound reads as stale with a zero count.
    let days = chrono::DateTime::parse_from_rfc3339(&snapshot.window_end)
        .map(|parsed| (now - parsed.with_timezone(&Utc)).num_days().max(0))
        .unwrap_or(0);
    ReadinessReason {
        reason: NotReadyReason::StaleWindow,
        detail: format!(
            "window `{}`..`{}` ended {days} day(s) ago and reads as stale against a {stale_after_days} day bound",
            snapshot.window_start, snapshot.window_end
        ),
    }
}

/// Select the candidate snapshot: current snapshots reporting the
/// metric, narrowed by optional source and normalized window, maximum
/// by `(window_end, id)`. No fallback to an older window on failure.
fn select_candidate<'a>(
    snapshots: &'a [InterestSnapshot],
    metric: InterestMetric,
    source: Option<&str>,
    requested_window: Option<(&str, &str)>,
) -> Option<&'a InterestSnapshot> {
    let mut candidates: Vec<&InterestSnapshot> = snapshots
        .iter()
        .filter(|snapshot| snapshot.metric(metric).is_some())
        .filter(|snapshot| source.map(|s| snapshot.source == s).unwrap_or(true))
        .filter(|snapshot| {
            requested_window
                .map(|(start, end)| snapshot.window_start == start && snapshot.window_end == end)
                .unwrap_or(true)
        })
        .collect();
    candidates.sort_by(|a, b| (&a.window_end, a.id).cmp(&(&b.window_end, b.id)));
    candidates.into_iter().last()
}

/// Build one project's verdict. `counts` is `(total, current)` and
/// `snapshots` carries the current snapshots only.
#[allow(clippy::too_many_arguments)]
pub fn build_readiness(
    project_id: &str,
    metric: InterestMetric,
    threshold: Option<u64>,
    source: Option<&str>,
    requested_window: Option<(&str, &str)>,
    counts: (usize, usize),
    snapshots: &[InterestSnapshot],
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> ReadinessVerdict {
    let mut reasons: Vec<ReadinessReason> = Vec::new();
    if threshold.is_none() {
        reasons.push(threshold_not_declared());
    }
    let candidate = if counts.0 == 0 {
        reasons.push(ReadinessReason {
            reason: NotReadyReason::NoEvidence,
            detail: format!(
                "project `{project_id}` has no stored interest snapshot; Forge reports no figure rather than a zero"
            ),
        });
        None
    } else if counts.1 == 0 {
        reasons.push(ReadinessReason {
            reason: NotReadyReason::SupersededOnly,
            detail: format!(
                "every stored snapshot for `{project_id}` has been superseded; history is not current evidence"
            ),
        });
        None
    } else {
        match select_candidate(snapshots, metric, source, requested_window) {
            Some(snapshot) => Some(snapshot),
            None => {
                let mut detail = format!("no current window reports `{}`", metric.label());
                if let Some((start, end)) = requested_window {
                    detail.push_str(&format!(" matching `{start}`..`{end}`"));
                }
                if let Some(source) = source {
                    detail.push_str(&format!(" from source `{source}`"));
                }
                reasons.push(ReadinessReason {
                    reason: NotReadyReason::NoCurrentWindow,
                    detail,
                });
                None
            }
        }
    };
    let mut notes: Vec<String> = Vec::new();
    let evidence = candidate.map(|snapshot| {
        // Other sources reporting the same window are noted, never merged.
        let mut others: Vec<(&str, &str)> = snapshots
            .iter()
            .filter(|other| {
                other.id != snapshot.id
                    && other.window_start == snapshot.window_start
                    && other.window_end == snapshot.window_end
                    && other.metric(metric).is_some()
            })
            .map(|other| (other.source.as_str(), other.source_revision.as_str()))
            .collect();
        others.sort_unstable();
        others.dedup();
        for (other_source, _) in &others {
            if Some(*other_source) != source && *other_source != snapshot.source.as_str() {
                notes.push(format!(
                    "another source `{other_source}` also reports this window; the verdict rests on source `{}` revision `{}`",
                    snapshot.source, snapshot.source_revision
                ));
            }
        }
        if snapshot.freshness(now, stale_after_days)
            == crate::portfolio::interest::Freshness::Stale
        {
            reasons.push(stale_reason(snapshot, now, stale_after_days));
        }
        if snapshot.privacy_mode != crate::portfolio::interest::PrivacyMode::ExactCount.label() {
            reasons.push(ReadinessReason {
                reason: NotReadyReason::InexactPrivacyMode,
                detail: format!(
                    "privacy mode `{}` is not an exact count; the figure is a floor or an undeclared measurement, not a headcount",
                    snapshot.privacy_mode
                ),
            });
        }
        if snapshot.coverage != crate::portfolio::interest::Coverage::Complete.label() {
            reasons.push(ReadinessReason {
                reason: NotReadyReason::PartialCoverage,
                detail: "coverage is `partial`; the source measured only part of the window, so the figure is not a complete measurement".to_string(),
            });
        }
        if let Some(threshold) = threshold {
            let value = snapshot
                .metric(metric)
                .expect("candidates report the metric");
            if value < threshold {
                reasons.push(ReadinessReason {
                    reason: NotReadyReason::BelowThreshold,
                    detail: format!(
                        "value {value} is below the declared threshold {threshold}"
                    ),
                });
            }
        }
        ReadinessEvidence {
            snapshot_id: snapshot.id,
            window_start: snapshot.window_start.clone(),
            window_end: snapshot.window_end.clone(),
            source: snapshot.source.clone(),
            source_revision: snapshot.source_revision.clone(),
            privacy_mode: snapshot.privacy_mode.clone(),
            coverage: snapshot.coverage.clone(),
            freshness: snapshot
                .freshness(now, stale_after_days)
                .label()
                .to_string(),
            value: snapshot.metric(metric).expect("candidates report the metric"),
        }
    });
    let readiness = if reasons.is_empty() {
        Readiness::Ready
    } else {
        Readiness::NotReady
    };
    ReadinessVerdict {
        project_id: project_id.to_string(),
        readiness,
        metric: metric.label().to_string(),
        threshold,
        reasons,
        evidence,
        notes,
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::portfolio::interest::activation::readiness::ActivationReport;
    use crate::portfolio::interest::{
        Coverage, InterestMetric, MetricValue, PrivacyMode, SnapshotState,
    };

    fn now() -> DateTime<Utc> {
        "2026-09-20T00:00:00Z".parse().expect("now")
    }

    fn snapshot(
        id: i64,
        source: &str,
        start: &str,
        end: &str,
        privacy: PrivacyMode,
        coverage: Coverage,
        value: u64,
    ) -> InterestSnapshot {
        InterestSnapshot {
            id,
            project_id: "alethefy".to_string(),
            source: source.to_string(),
            source_revision: format!("rev-{id}"),
            window_start: start.to_string(),
            window_end: end.to_string(),
            privacy_mode: privacy.label().to_string(),
            coverage: coverage.label().to_string(),
            state: SnapshotState::Accepted,
            replaces_source_revision: None,
            actor: "ops".to_string(),
            received_at: end.to_string(),
            metrics: vec![MetricValue {
                metric: InterestMetric::UniqueVisitors.label().to_string(),
                value,
            }],
        }
    }

    fn current_window() -> InterestSnapshot {
        snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            120,
        )
    }

    #[test]
    fn the_readiness_vocabulary_is_closed() {
        assert_eq!(Readiness::labels(), vec!["ready", "not-ready"]);
        assert!(Readiness::parse("ready").unwrap().is_ready());
        assert!(!Readiness::parse("not-ready").unwrap().is_ready());
        assert!(Readiness::parse("maybe").is_err());
    }

    #[test]
    fn the_reason_vocabulary_is_closed_and_in_report_order() {
        assert_eq!(
            NotReadyReason::labels(),
            vec![
                "threshold-not-declared",
                "no-evidence",
                "superseded-only",
                "no-current-window",
                "stale-window",
                "inexact-privacy-mode",
                "partial-coverage",
                "below-threshold",
            ]
        );
        // Declaration order is the report order: the enum derives Ord
        // in declaration order.
        let mut ordered = NotReadyReason::ALL.to_vec();
        ordered.sort();
        assert_eq!(ordered, NotReadyReason::ALL.to_vec());
    }

    #[test]
    fn thresholds_are_bounded_and_zero_is_legal() {
        assert_eq!(validate_threshold(0).expect("zero is legal"), 0);
        assert_eq!(
            validate_threshold(MAX_METRIC_VALUE).expect("bound is legal"),
            MAX_METRIC_VALUE
        );
        assert!(validate_threshold(MAX_METRIC_VALUE + 1).is_err());
    }

    #[test]
    fn a_declared_window_parses_normalizes_and_requires_order() {
        assert_eq!(
            parse_window("2026-09-01T02:00:00+02:00..2026-09-08T00:00:00Z").expect("window"),
            (
                "2026-09-01T00:00:00Z".to_string(),
                "2026-09-08T00:00:00Z".to_string()
            )
        );
        assert!(parse_window("2026-09-01T00:00:00Z").is_err());
        assert!(parse_window("not-a-time..2026-09-08T00:00:00Z").is_err());
        assert!(parse_window("2026-09-08T00:00:00Z..2026-09-01T00:00:00Z").is_err());
        assert!(parse_window("2026-09-08T00:00:00Z..2026-09-08T00:00:00Z").is_err());
    }

    #[test]
    fn no_evidence_is_not_ready_and_invents_no_zero() {
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (0, 0),
            &[],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert!(verdict.evidence.is_none());
        assert_eq!(verdict.reasons.len(), 1);
        assert_eq!(verdict.reasons[0].reason, NotReadyReason::NoEvidence);
        let rendered = serde_json::to_string(&verdict).expect("render");
        assert!(!rendered.contains("\"value\":0"), "{rendered}");
    }

    #[test]
    fn superseded_only_evidence_is_not_ready() {
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 0),
            &[],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert!(verdict.evidence.is_none());
        assert_eq!(
            verdict.reasons.iter().map(|r| r.reason).collect::<Vec<_>>(),
            vec![NotReadyReason::SupersededOnly]
        );
    }

    #[test]
    fn a_current_window_without_the_metric_is_no_current_window() {
        let other = InterestSnapshot {
            metrics: vec![MetricValue {
                metric: InterestMetric::OutboundCtaClicks.label().to_string(),
                value: 9,
            }],
            ..current_window()
        };
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[other],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert!(verdict.evidence.is_none());
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::NoCurrentWindow));
        assert!(verdict.reasons[0].detail.contains("unique_visitors"));
    }

    #[test]
    fn the_latest_reported_window_is_the_one_evaluated() {
        let old = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            500,
        );
        let mut stale = snapshot(
            2,
            "github-analytics",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            5,
        );
        stale.window_end = "2026-08-10T00:00:00Z".to_string();
        stale.window_start = "2026-08-03T00:00:00Z".to_string();
        // Two disjoint current windows: the later end wins even though
        // the older one would satisfy the threshold.
        let fresh_old = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            5,
        );
        let fresh_new = snapshot(
            2,
            "github-analytics",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            500,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (2, 2),
            &[fresh_old, fresh_new.clone()],
            now(),
            90,
        );
        assert_eq!(verdict.readiness, Readiness::Ready);
        assert_eq!(verdict.evidence.as_ref().expect("evidence").snapshot_id, 2);
        let _ = (old, stale);
    }

    #[test]
    fn a_declared_window_narrows_the_evidence() {
        let first = current_window();
        let second = snapshot(
            2,
            "github-analytics",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            3,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            Some(("2026-09-08T00:00:00Z", "2026-09-15T00:00:00Z")),
            (2, 2),
            &[first, second],
            now(),
            90,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert_eq!(verdict.evidence.as_ref().expect("evidence").snapshot_id, 2);
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::BelowThreshold));
        // A window that matches nothing is no-current-window.
        let missing = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            Some(("2026-10-01T00:00:00Z", "2026-10-08T00:00:00Z")),
            (2, 2),
            &[current_window()],
            now(),
            90,
        );
        assert!(missing
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::NoCurrentWindow));
        assert!(missing.reasons[0].detail.contains("2026-10-01"));
    }

    #[test]
    fn a_declared_source_narrows_the_evidence() {
        let github = current_window();
        let mut other = current_window();
        other.id = 2;
        other.source = "content-analytics".to_string();
        other.source_revision = "rev-2".to_string();
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            Some("content-analytics"),
            None,
            (2, 2),
            &[github, other],
            now(),
            14,
        );
        assert_eq!(
            verdict.evidence.as_ref().expect("evidence").source,
            "content-analytics"
        );
        let missing = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            Some("no-such-source"),
            None,
            (2, 2),
            &[current_window()],
            now(),
            14,
        );
        assert!(missing
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::NoCurrentWindow));
        assert!(missing.reasons[0].detail.contains("no-such-source"));
    }

    #[test]
    fn a_stale_window_is_not_ready_even_above_threshold() {
        let stale = snapshot(
            1,
            "github-analytics",
            "2026-08-01T00:00:00Z",
            "2026-08-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            500,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[stale],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::StaleWindow));
        assert!(verdict
            .reasons
            .iter()
            .all(|r| r.reason != NotReadyReason::BelowThreshold));
    }

    #[test]
    fn a_lower_bound_figure_is_never_a_headcount() {
        let lower = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::LowerBound,
            Coverage::Complete,
            500,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[lower],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::InexactPrivacyMode));
        assert!(verdict.reasons[0].detail.contains("lower-bound"));
    }

    #[test]
    fn an_undeclared_privacy_mode_is_not_ready() {
        let undeclared = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::Undeclared,
            Coverage::Complete,
            500,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[undeclared],
            now(),
            14,
        );
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::InexactPrivacyMode));
    }

    #[test]
    fn partial_coverage_is_not_ready() {
        let partial = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Partial,
            500,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[partial],
            now(),
            14,
        );
        assert!(verdict
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::PartialCoverage));
    }

    #[test]
    fn an_all_zero_complete_window_can_be_ready_at_threshold_zero() {
        let zero = snapshot(
            1,
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            PrivacyMode::ExactCount,
            Coverage::Complete,
            0,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(0),
            None,
            None,
            (1, 1),
            &[zero],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::Ready);
        assert!(verdict.reasons.is_empty());
    }

    #[test]
    fn below_threshold_is_not_ready_at_the_exact_boundary() {
        let at = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(120),
            None,
            None,
            (1, 1),
            &[current_window()],
            now(),
            14,
        );
        assert_eq!(at.readiness, Readiness::Ready);
        let below = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(121),
            None,
            None,
            (1, 1),
            &[current_window()],
            now(),
            14,
        );
        assert_eq!(below.readiness, Readiness::NotReady);
        assert!(below
            .reasons
            .iter()
            .any(|r| r.reason == NotReadyReason::BelowThreshold));
        assert!(below.reasons[0].detail.contains("120"));
    }

    #[test]
    fn every_withholding_condition_is_reported_not_collapsed() {
        let bad = snapshot(
            1,
            "github-analytics",
            "2026-08-01T00:00:00Z",
            "2026-08-08T00:00:00Z",
            PrivacyMode::LowerBound,
            Coverage::Partial,
            5,
        );
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            None,
            None,
            None,
            (1, 1),
            &[bad],
            now(),
            14,
        );
        let reasons: Vec<NotReadyReason> = verdict.reasons.iter().map(|r| r.reason).collect();
        assert_eq!(
            reasons,
            vec![
                NotReadyReason::ThresholdNotDeclared,
                NotReadyReason::StaleWindow,
                NotReadyReason::InexactPrivacyMode,
                NotReadyReason::PartialCoverage,
            ]
        );
    }

    #[test]
    fn an_absent_threshold_is_a_verdict_not_an_error() {
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            None,
            None,
            None,
            (1, 1),
            &[current_window()],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::NotReady);
        assert_eq!(verdict.reasons.len(), 1);
        assert_eq!(
            verdict.reasons[0].reason,
            NotReadyReason::ThresholdNotDeclared
        );
    }

    #[test]
    fn the_two_sources_sharing_a_window_add_a_note_and_settle_on_the_latest() {
        let mut github = current_window();
        github.id = 1;
        let mut other = current_window();
        other.id = 2;
        other.source = "content-analytics".to_string();
        other.source_revision = "rev-2".to_string();
        let verdict = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (2, 2),
            &[github, other],
            now(),
            14,
        );
        assert_eq!(verdict.readiness, Readiness::Ready);
        assert_eq!(verdict.evidence.as_ref().expect("evidence").snapshot_id, 2);
        assert_eq!(verdict.notes.len(), 1);
        assert!(
            verdict.notes[0].contains("content-analytics")
                || verdict.notes[0].contains("github-analytics")
        );
        assert!(verdict.notes[0].contains("the verdict rests on source"));
    }

    #[test]
    fn the_verdict_is_deterministic_for_repeated_reads() {
        let first = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[current_window()],
            now(),
            14,
        );
        let second = build_readiness(
            "alethefy",
            InterestMetric::UniqueVisitors,
            Some(100),
            None,
            None,
            (1, 1),
            &[current_window()],
            now(),
            14,
        );
        assert_eq!(first, second);
        let report = ActivationReport {
            metric: InterestMetric::UniqueVisitors.label().to_string(),
            threshold: Some(100),
            stale_after_days: 14,
            requested_window: None,
            requested_source: None,
            verdicts: vec![first, second.clone()],
            ready_count: 2,
            not_ready_count: 0,
        };
        assert!(report.is_ready());
        assert!(report.verdict_reason_labels().is_empty());
    }
}
