//! How interest evidence is read back.
//!
//! The read model exists to answer one question — *which project is
//! worth more attention* — without ever answering it dishonestly. That
//! rules out the three moves a comparison normally reaches for:
//!
//! - **No totals.** A sum across two windows counts the same visitor
//!   twice, so there is no `total` field to fill in, at any level.
//! - **No ranking across mismatched windows.** When two projects did
//!   not report the same window, the comparison says so and
//!   [`Comparison::comparable`] goes false rather than ordering them
//!   anyway.
//! - **No laundering of staleness.** Freshness is derived from the
//!   window end at read time and travels with every row, so a stale
//!   figure is never rendered as the present one.
//!
//! What survives is the boring, checkable version: one row per
//! (project, metric, window), each carrying its source, its source
//! revision, how the source suppressed cohorts, and how long ago the
//! window ended.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::portfolio::interest::{
    parse_timestamp, InterestMetric, InterestSnapshot, MAX_STALE_AFTER_DAYS,
};

// --- freshness ----------------------------------------------------------

/// Whether a stored window still describes the present.
///
/// `Stale` is not a lesser state that Forge upgrades or hides: the row
/// keeps its value, keeps its source and keeps its provenance, and
/// simply refuses to be called current. The bound is applied at read
/// time so that widening it never rewrites history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Freshness {
    Current,
    Stale,
}

impl Freshness {
    pub fn label(&self) -> &'static str {
        match self {
            Freshness::Current => "current",
            Freshness::Stale => "stale",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "current" => Ok(Freshness::Current),
            "stale" => Ok(Freshness::Stale),
            _ => Err(format!(
                "unknown freshness `{raw}`; expected one of current, stale"
            )),
        }
    }

    pub fn is_current(&self) -> bool {
        matches!(self, Freshness::Current)
    }
}

/// Bound one staleness value, refusing an out-of-range declaration
/// rather than silently clamping it: an operator who asked for zero
/// days of tolerance should be told no, not given the default.
pub fn bound_stale_after_days(raw: i64) -> Result<i64, String> {
    if !(1..=MAX_STALE_AFTER_DAYS).contains(&raw) {
        return Err(format!(
            "stale_after_days must be between 1 and {MAX_STALE_AFTER_DAYS}"
        ));
    }
    Ok(raw)
}

/// Label one window end against a reference time.
///
/// A window that has not ended yet is `current` — it is still
/// accruing, not stale — and a window whose end cannot be read is
/// `stale`, which is the conservative reading: an unreadable bound
/// must never present itself as the present.
pub fn freshness_label(window_end: &str, now: DateTime<Utc>, stale_after_days: i64) -> Freshness {
    match parse_timestamp(window_end) {
        Ok(end) if end > now => Freshness::Current,
        Ok(end) if (now - end).num_days() < stale_after_days => Freshness::Current,
        Ok(_) => Freshness::Stale,
        Err(_) => Freshness::Stale,
    }
}

// --- comparison ---------------------------------------------------------

/// One project's figure for one metric in one window.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRow {
    pub project_id: String,
    pub metric: String,
    pub value: u64,
    /// The UTC half-open window this figure covers, normalized.
    pub window_start: String,
    pub window_end: String,
    /// The analytics source that produced it.
    pub source: String,
    /// The revision of that source the figure belongs to.
    pub source_revision: String,
    /// How the source produced the figure. `exact-count` may be
    /// compared as an exact number; `lower-bound` and `undeclared`
    /// may not.
    pub privacy_mode: String,
    /// Whether the source measured the whole window.
    pub coverage: String,
    /// Whether this window still describes the present.
    pub freshness: String,
}

/// A privacy-safe comparison across projects.
///
/// There is deliberately no total, average or rank. [`rows`] is the
/// whole answer, and when the projects did not report comparable
/// windows [`comparable`] is false and [`notes`] says so.
#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
    pub metrics: Vec<String>,
    pub rows: Vec<ComparisonRow>,
    /// Every distinct window present in the evidence, oldest first.
    pub windows: Vec<String>,
    /// Whether every row reports the same window. When false, the
    /// rows are shown side by side rather than ranked.
    pub comparable: bool,
    /// Whether any row is stale. A comparison containing stale
    /// evidence is still shown, never silently presented as current.
    pub contains_stale: bool,
    /// Human-readable statements of what this comparison will not do.
    pub notes: Vec<String>,
}

/// Build a comparison from already-read snapshots.
///
/// Snapshots are expected to be the *current* ones only — a superseded
/// revision is history, and showing it beside its replacement would
/// present two answers to the same question.
pub fn build_comparison(
    snapshots: &[InterestSnapshot],
    metrics: &[InterestMetric],
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> Comparison {
    let mut rows: Vec<ComparisonRow> = Vec::new();
    let mut windows: BTreeSet<String> = BTreeSet::new();
    let mut contains_stale = false;

    for snapshot in snapshots {
        let freshness = snapshot.freshness(now, stale_after_days);
        contains_stale |= freshness == Freshness::Stale;
        for metric in metrics {
            let Some(value) = snapshot.metric(*metric) else {
                // A metric the source did not report is absent, not
                // zero. Emitting a zero here would invent an
                // observation the provider never made.
                continue;
            };
            windows.insert(format!(
                "{}..{}",
                snapshot.window_start, snapshot.window_end
            ));
            rows.push(ComparisonRow {
                project_id: snapshot.project_id.clone(),
                metric: metric.label().to_string(),
                value,
                window_start: snapshot.window_start.clone(),
                window_end: snapshot.window_end.clone(),
                source: snapshot.source.clone(),
                source_revision: snapshot.source_revision.clone(),
                privacy_mode: snapshot.privacy_mode.clone(),
                coverage: snapshot.coverage.clone(),
                freshness: freshness.label().to_string(),
            });
        }
    }
    // Canonical order: project, then metric, then window. Two reads of
    // the same evidence produce byte-identical output.
    rows.sort_by(|a, b| {
        (&a.project_id, &a.metric, &a.window_start, &a.source).cmp(&(
            &b.project_id,
            &b.metric,
            &b.window_start,
            &b.source,
        ))
    });

    let window_list: Vec<String> = windows.into_iter().collect();
    let comparable = window_list.len() <= 1;

    let mut notes = Vec::new();
    if !comparable {
        notes.push(format!(
            "snapshots span {} different windows; Forge will not rank or total across windows",
            window_list.len()
        ));
    }
    if contains_stale {
        notes.push(
            "at least one window is stale; its row is labelled `stale` and is not presented \
             as the present measurement"
                .to_string(),
        );
    }
    if rows.is_empty() {
        notes.push(
            "no project reported any of the requested metrics; Forge reports no figure rather \
             than a zero it did not measure"
                .to_string(),
        );
    }
    let inexact: BTreeSet<&str> = snapshots
        .iter()
        .map(|snapshot| snapshot.privacy_mode.as_str())
        .filter(|mode| *mode != "exact-count")
        .collect();
    if !inexact.is_empty() {
        let mut names: Vec<&str> = inexact.into_iter().collect();
        names.sort_unstable();
        notes.push(format!(
            "privacy mode {} is not an exact count; the figure is a floor or an undeclared \
             measurement, not a headcount",
            names.join(", ")
        ));
    }

    Comparison {
        metrics: metrics.iter().map(|m| m.label().to_string()).collect(),
        rows,
        windows: window_list,
        comparable,
        contains_stale,
        notes,
    }
}

// --- trend --------------------------------------------------------------

/// One windowed point in a metric's history.
#[derive(Debug, Clone, Serialize)]
pub struct TrendPoint {
    pub window_start: String,
    pub window_end: String,
    pub value: u64,
    pub privacy_mode: String,
    pub coverage: String,
    pub freshness: String,
    pub source: String,
}

/// One project's history for one metric.
#[derive(Debug, Clone, Serialize)]
pub struct Trend {
    pub project_id: String,
    pub metric: String,
    /// Oldest window first. Points are only the windows that actually
    /// reported the metric.
    pub points: Vec<TrendPoint>,
    /// How many stored windows did not report this metric at all.
    /// They are counted rather than plotted as zero.
    pub unreported_windows: usize,
    /// What this series does not establish.
    pub note: String,
}

/// Build a chronological series from already-read snapshots.
///
/// The series is a list of windows, not a fitted line: Forge has no
/// business extrapolating an interest signal past the last window a
/// source actually reported.
pub fn build_trend(
    project_id: &str,
    metric: InterestMetric,
    snapshots: &[InterestSnapshot],
    limit: usize,
    now: DateTime<Utc>,
    stale_after_days: i64,
) -> Trend {
    let mut points: Vec<TrendPoint> = Vec::new();
    let mut unreported = 0usize;
    // Oldest first, so a bounded series keeps the history rather than
    // the most recent readings.
    let mut ordered: Vec<&InterestSnapshot> = snapshots.iter().collect();
    ordered.sort_by(|a, b| (&a.window_start, &a.id).cmp(&(&b.window_start, &b.id)));

    for snapshot in ordered {
        let Some(value) = snapshot.metric(metric) else {
            unreported += 1;
            continue;
        };
        points.push(TrendPoint {
            window_start: snapshot.window_start.clone(),
            window_end: snapshot.window_end.clone(),
            value,
            privacy_mode: snapshot.privacy_mode.clone(),
            coverage: snapshot.coverage.clone(),
            freshness: snapshot
                .freshness(now, stale_after_days)
                .label()
                .to_string(),
            source: snapshot.source.clone(),
        });
    }
    if points.len() > limit {
        points.drain(..points.len() - limit);
    }

    let note = if points.is_empty() {
        format!(
            "no stored window reported `{}`; Forge plots no point rather than inventing one",
            metric.label()
        )
    } else {
        "each point is one window a source reported; the series is not a projection, and Forge \
         does not interpolate, sum or extrapolate across windows"
            .to_string()
    };

    Trend {
        project_id: project_id.to_string(),
        metric: metric.label().to_string(),
        points,
        unreported_windows: unreported,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::interest::{MetricValue, SnapshotState};

    fn now() -> DateTime<Utc> {
        "2026-09-20T00:00:00Z".parse().expect("now")
    }

    fn snapshot(
        project: &str,
        source: &str,
        start: &str,
        end: &str,
        privacy: &str,
        values: &[(&str, u64)],
    ) -> InterestSnapshot {
        InterestSnapshot {
            id: 1,
            project_id: project.to_string(),
            source: source.to_string(),
            source_revision: "rev-1".to_string(),
            window_start: start.to_string(),
            window_end: end.to_string(),
            privacy_mode: privacy.to_string(),
            coverage: "complete".to_string(),
            state: SnapshotState::Accepted,
            replaces_source_revision: None,
            actor: "ops".to_string(),
            received_at: end.to_string(),
            metrics: values
                .iter()
                .map(|(metric, value)| MetricValue {
                    metric: (*metric).to_string(),
                    value: *value,
                })
                .collect(),
        }
    }

    #[test]
    fn freshness_is_derived_from_the_window_end_at_read_time() {
        assert_eq!(
            freshness_label("2026-09-19T00:00:00Z", now(), 14),
            Freshness::Current
        );
        assert_eq!(
            freshness_label("2026-09-06T00:00:00Z", now(), 14),
            Freshness::Stale
        );
        // A window that has not ended yet is still accruing.
        assert_eq!(
            freshness_label("2026-09-30T00:00:00Z", now(), 14),
            Freshness::Current
        );
        // An unreadable bound is never presented as the present.
        assert_eq!(freshness_label("not-a-date", now(), 14), Freshness::Stale);
        // The bound is a bound, not a clamp.
        assert_eq!(
            freshness_label("2026-09-06T00:00:00Z", now(), 30),
            Freshness::Current
        );
        assert_eq!(Freshness::parse("stale").unwrap().label(), "stale");
        assert!(Freshness::parse("fresh").is_err());
        assert_eq!(bound_stale_after_days(14).expect("bound"), 14);
        assert!(bound_stale_after_days(0).is_err());
        assert!(bound_stale_after_days(366).is_err());
    }

    #[test]
    fn a_comparison_never_sums_and_says_so_when_windows_differ() {
        let alpha = snapshot(
            "alpha",
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            "exact-count",
            &[("unique_visitors", 100)],
        );
        let beta = snapshot(
            "beta",
            "github-analytics",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
            "exact-count",
            &[("unique_visitors", 40)],
        );
        let comparison = build_comparison(
            &[alpha.clone(), beta.clone()],
            &[InterestMetric::UniqueVisitors],
            now(),
            14,
        );
        assert_eq!(comparison.rows.len(), 2);
        assert!(!comparison.comparable, "two windows are not comparable");
        assert_eq!(comparison.windows.len(), 2);
        assert!(comparison
            .notes
            .iter()
            .any(|note| note.contains("will not rank or total")));
        // 100 and 40 are never added into 140 anywhere in the shape.
        let rendered = serde_json::to_string(&comparison).expect("render");
        assert!(!rendered.contains("140"), "{rendered}");

        // The same window on both sides is comparable.
        let mut beta_same = beta;
        beta_same.window_start = "2026-09-01T00:00:00Z".to_string();
        beta_same.window_end = "2026-09-08T00:00:00Z".to_string();
        let aligned = build_comparison(
            &[alpha, beta_same],
            &[InterestMetric::UniqueVisitors],
            now(),
            14,
        );
        assert!(aligned.comparable);
        assert_eq!(aligned.windows.len(), 1);
    }

    #[test]
    fn an_unreported_metric_is_absent_and_never_a_zero() {
        let alpha = snapshot(
            "alpha",
            "github-analytics",
            "2026-09-10T00:00:00Z",
            "2026-09-17T00:00:00Z",
            "exact-count",
            &[("unique_visitors", 100)],
        );
        let comparison = build_comparison(
            std::slice::from_ref(&alpha),
            &[
                InterestMetric::UniqueVisitors,
                InterestMetric::PaidInterestEvents,
            ],
            now(),
            14,
        );
        assert_eq!(
            comparison.rows.len(),
            1,
            "only the reported metric has a row"
        );
        assert!(comparison
            .rows
            .iter()
            .all(|row| row.metric == "unique_visitors"));
        // The requested metric is named in the request echo, but it
        // never acquires a row: absence is not a zero.
        assert!(comparison
            .rows
            .iter()
            .all(|row| row.metric != "paid_interest_events"));
    }

    #[test]
    fn a_comparison_carries_stale_and_privacy_labels_into_every_row() {
        let old = snapshot(
            "alpha",
            "github-analytics",
            "2026-08-01T00:00:00Z",
            "2026-08-08T00:00:00Z",
            "lower-bound",
            &[("unique_visitors", 100)],
        );
        let comparison = build_comparison(&[old], &[InterestMetric::UniqueVisitors], now(), 14);
        assert!(comparison.contains_stale);
        assert_eq!(comparison.rows[0].freshness, "stale");
        assert_eq!(comparison.rows[0].privacy_mode, "lower-bound");
        assert!(comparison.notes.iter().any(|note| note.contains("stale")));
        assert!(comparison
            .notes
            .iter()
            .any(|note| note.contains("lower-bound")));
    }

    #[test]
    fn a_trend_orders_windows_and_counts_the_ones_that_reported_nothing() {
        let mut third = snapshot(
            "alpha",
            "github-analytics",
            "2026-09-15T00:00:00Z",
            "2026-09-22T00:00:00Z",
            "exact-count",
            &[("unique_visitors", 30)],
        );
        third.id = 3;
        let mut first = snapshot(
            "alpha",
            "github-analytics",
            "2026-09-01T00:00:00Z",
            "2026-09-08T00:00:00Z",
            "exact-count",
            &[("unique_visitors", 10)],
        );
        first.id = 1;
        let mut gap = snapshot(
            "alpha",
            "github-analytics",
            "2026-09-08T00:00:00Z",
            "2026-09-15T00:00:00Z",
            "exact-count",
            &[("completed_public_workflows", 4)],
        );
        gap.id = 2;

        // Deliberately out of order: the builder sorts by window.
        let trend = build_trend(
            "alpha",
            InterestMetric::UniqueVisitors,
            &[third, first.clone(), gap],
            10,
            now(),
            14,
        );
        assert_eq!(
            trend
                .points
                .iter()
                .map(|p| p.window_start.as_str())
                .collect::<Vec<_>>(),
            vec!["2026-09-01T00:00:00Z", "2026-09-15T00:00:00Z"]
        );
        assert_eq!(
            trend.unreported_windows, 1,
            "the gap is counted, not zeroed"
        );
        assert!(trend.note.contains("not a projection"));

        let empty = build_trend(
            "alpha",
            InterestMetric::OutboundCtaClicks,
            &[first],
            10,
            now(),
            14,
        );
        assert!(empty.points.is_empty());
        assert!(empty.note.contains("inventing one"));
    }

    #[test]
    fn a_bounded_trend_keeps_the_most_recent_windows() {
        let snapshots: Vec<InterestSnapshot> = (0..5)
            .map(|index| {
                let mut entry = snapshot(
                    "alpha",
                    "github-analytics",
                    &format!("2026-09-0{}T00:00:00Z", index + 1),
                    &format!("2026-09-1{}T00:00:00Z", index),
                    "exact-count",
                    &[("unique_visitors", index as u64 * 10)],
                );
                entry.id = index + 1;
                entry
            })
            .collect();
        let trend = build_trend(
            "alpha",
            InterestMetric::UniqueVisitors,
            &snapshots,
            2,
            now(),
            14,
        );
        assert_eq!(trend.points.len(), 2);
        assert_eq!(trend.points[0].window_start, "2026-09-04T00:00:00Z");
        assert_eq!(trend.points[1].window_start, "2026-09-05T00:00:00Z");
    }
}
