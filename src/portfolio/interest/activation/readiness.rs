//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;
use std::collections::BTreeSet;

/// Whether one project's aggregate evidence justifies the
/// product-owned activation follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness {
    Ready,
    NotReady,
}
impl Readiness {
    pub const ALL: [Readiness; 2] = [Readiness::Ready, Readiness::NotReady];
    pub fn label(&self) -> &'static str {
        match self {
            Readiness::Ready => "ready",
            Readiness::NotReady => "not-ready",
        }
    }
    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown readiness `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }
    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
    pub fn is_ready(&self) -> bool {
        matches!(self, Readiness::Ready)
    }
}
/// Why readiness is withheld. Declaration order **is** the report
/// order: the evaluation pushes in this order and never sorts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotReadyReason {
    ThresholdNotDeclared,
    NoEvidence,
    SupersededOnly,
    NoCurrentWindow,
    StaleWindow,
    InexactPrivacyMode,
    PartialCoverage,
    BelowThreshold,
}
impl NotReadyReason {
    pub const ALL: [NotReadyReason; 8] = [
        NotReadyReason::ThresholdNotDeclared,
        NotReadyReason::NoEvidence,
        NotReadyReason::SupersededOnly,
        NotReadyReason::NoCurrentWindow,
        NotReadyReason::StaleWindow,
        NotReadyReason::InexactPrivacyMode,
        NotReadyReason::PartialCoverage,
        NotReadyReason::BelowThreshold,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            NotReadyReason::ThresholdNotDeclared => "threshold-not-declared",
            NotReadyReason::NoEvidence => "no-evidence",
            NotReadyReason::SupersededOnly => "superseded-only",
            NotReadyReason::NoCurrentWindow => "no-current-window",
            NotReadyReason::StaleWindow => "stale-window",
            NotReadyReason::InexactPrivacyMode => "inexact-privacy-mode",
            NotReadyReason::PartialCoverage => "partial-coverage",
            NotReadyReason::BelowThreshold => "below-threshold",
        }
    }
    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}
/// One project's verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessVerdict {
    pub project_id: String,
    pub readiness: Readiness,
    pub metric: String,
    pub threshold: Option<u64>,
    pub reasons: Vec<ReadinessReason>,
    pub evidence: Option<ReadinessEvidence>,
    pub notes: Vec<String>,
}
/// A declared period, both sides normalized to UTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestedWindow {
    pub start: String,
    pub end: String,
}
/// The fleet readiness report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivationReport {
    pub metric: String,
    pub threshold: Option<u64>,
    pub stale_after_days: i64,
    pub requested_window: Option<RequestedWindow>,
    pub requested_source: Option<String>,
    pub verdicts: Vec<ReadinessVerdict>,
    pub ready_count: usize,
    pub not_ready_count: usize,
}
impl ActivationReport {
    pub fn is_ready(&self) -> bool {
        !self.verdicts.is_empty() && self.not_ready_count == 0
    }
    pub fn ready_count(&self) -> usize {
        self.verdicts
            .iter()
            .filter(|v| v.readiness.is_ready())
            .count()
    }
    pub fn not_ready_count(&self) -> usize {
        self.verdicts
            .iter()
            .filter(|v| !v.readiness.is_ready())
            .count()
    }
    /// One `` `{project_id}:{reason}` `` label per distinct reason
    /// across all verdicts, deduplicated and sorted, for the bounded
    /// stderr line of the CLI gate.
    pub fn verdict_reason_labels(&self) -> Vec<String> {
        let mut labels: BTreeSet<String> = BTreeSet::new();
        for verdict in &self.verdicts {
            for reason in &verdict.reasons {
                labels.insert(format!("{}:{}", verdict.project_id, reason.reason.label()));
            }
        }
        labels.into_iter().collect()
    }
}
/// One named withholding condition with its human detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessReason {
    pub reason: NotReadyReason,
    pub detail: String,
}
/// The evidence one verdict rests on: the selected snapshot plus the
/// labels the reader needs to trust it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessEvidence {
    pub snapshot_id: i64,
    pub window_start: String,
    pub window_end: String,
    pub source: String,
    pub source_revision: String,
    pub privacy_mode: String,
    pub coverage: String,
    pub freshness: String,
    pub value: u64,
}
