//! Portfolio interest vocabularies: the closed value sets every
//! snapshot is built from.
//!
//! These four enums moved here verbatim from
//! [`crate::portfolio::interest`]; they are re-exported there, so
//! every `crate::portfolio::interest::<name>` path resolves exactly
//! as before.

use serde::Serialize;

// --- privacy mode --------------------------------------------------------

/// How the source produced the figures, and therefore what Forge may
/// claim about them.
///
/// This is the field that keeps a suppressed number from reading as an
/// exact one. `LowerBound` is not a smaller estimate — it means the
/// provider withheld cohorts below its own threshold, so the true
/// figure is at least this and never this exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyMode {
    /// The source counted the whole window and declared no suppression.
    ExactCount,
    /// The source suppressed small cohorts. The value is a floor.
    LowerBound,
    /// The source did not declare how it produced the figure, so
    /// Forge refuses to present it as an exact count.
    Undeclared,
}

impl PrivacyMode {
    pub const ALL: [PrivacyMode; 3] = [
        PrivacyMode::ExactCount,
        PrivacyMode::LowerBound,
        PrivacyMode::Undeclared,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PrivacyMode::ExactCount => "exact-count",
            PrivacyMode::LowerBound => "lower-bound",
            PrivacyMode::Undeclared => "undeclared",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown privacy mode `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    /// Whether a figure in this mode may be compared as an exact
    /// number. Only an explicitly declared exact count is; an
    /// undeclared one is never quietly treated as exact.
    pub fn is_exact(&self) -> bool {
        matches!(self, PrivacyMode::ExactCount)
    }
}

// --- coverage -----------------------------------------------------------

/// How much of the declared window the source actually measured.
///
/// This is what makes a zero meaningful. A source that says it
/// measured the complete window may legitimately report zero; a
/// source that measured part of it reporting zero is a collection
/// gap, and Forge refuses the record rather than storing an
/// observation that says "nobody was interested".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    /// The source measured the whole declared window.
    Complete,
    /// The source measured part of the declared window.
    Partial,
}

impl Coverage {
    pub const ALL: [Coverage; 2] = [Coverage::Complete, Coverage::Partial];

    pub fn label(&self) -> &'static str {
        match self {
            Coverage::Complete => "complete",
            Coverage::Partial => "partial",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown coverage `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- metrics ------------------------------------------------------------

/// The closed allowlist of aggregate metrics Forge will store.
///
/// Every entry is a count over one declared window. There is no
/// duration, no currency, no percentage and no free-form metric: an
/// unlisted key is refused, because an unlisted key is exactly where
/// an identity or a per-event payload would arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterestMetric {
    /// Distinct visitors the source counted in the window.
    UniqueVisitors,
    /// Publicly reachable workflows the visitor completed.
    CompletedPublicWorkflows,
    /// Visitors the source had already seen in an earlier window.
    ReturningVisitors,
    /// Outbound calls to action the source recorded.
    OutboundCtaClicks,
    /// Aggregate demand signal emitted by a product. Not a payment
    /// record and never a basis for granting access.
    PaidInterestEvents,
}

impl InterestMetric {
    pub const ALL: [InterestMetric; 5] = [
        InterestMetric::UniqueVisitors,
        InterestMetric::CompletedPublicWorkflows,
        InterestMetric::ReturningVisitors,
        InterestMetric::OutboundCtaClicks,
        InterestMetric::PaidInterestEvents,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            InterestMetric::UniqueVisitors => "unique_visitors",
            InterestMetric::CompletedPublicWorkflows => "completed_public_workflows",
            InterestMetric::ReturningVisitors => "returning_visitors",
            InterestMetric::OutboundCtaClicks => "outbound_cta_clicks",
            InterestMetric::PaidInterestEvents => "paid_interest_events",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown metric `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    /// What the figure means, in one clause. Rendered next to the
    /// value so a reader never has to guess whether `2` is people,
    /// clicks or workflows.
    pub fn unit(&self) -> &'static str {
        match self {
            InterestMetric::UniqueVisitors => "distinct visitors",
            InterestMetric::CompletedPublicWorkflows => "completed public workflows",
            InterestMetric::ReturningVisitors => "returning visitors",
            InterestMetric::OutboundCtaClicks => "outbound call-to-action clicks",
            InterestMetric::PaidInterestEvents => "aggregate paid-interest signals",
        }
    }
}

// --- snapshot state -----------------------------------------------------

/// Where one stored snapshot sits relative to its replacements.
///
/// Snapshots are never edited or deleted, so this is the only thing
/// that ever changes about a row: a source that re-measures a window
/// declares the revision it replaces, and the earlier snapshot is
/// marked superseded rather than overwritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotState {
    /// Current evidence for its window.
    Accepted,
    /// A later source revision replaced this one for the same window.
    Superseded,
}

impl SnapshotState {
    pub const ALL: [SnapshotState; 2] = [SnapshotState::Accepted, SnapshotState::Superseded];

    pub fn label(&self) -> &'static str {
        match self {
            SnapshotState::Accepted => "accepted",
            SnapshotState::Superseded => "superseded",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown snapshot state `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    /// Whether this snapshot may take part in a comparison. A
    /// superseded snapshot is history: reporting it beside its
    /// replacement would present two answers to one question.
    pub fn is_current(&self) -> bool {
        matches!(self, SnapshotState::Accepted)
    }
}
