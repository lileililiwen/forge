//! Portfolio vocabularies: the closed value sets the read model is built from.
//!
//! These four enums moved here verbatim from
//! [`crate::portfolio`]; they are re-exported there, so every
//! `crate::portfolio::<name>` path resolves exactly as before.

use serde::Serialize;

// --- lifecycle ---------------------------------------------------------

/// Where an operator says a project sits in its own lifecycle.
/// This is a *user classification*: it never overrides, and is
/// never derived from, a maturity or verification observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lifecycle {
    Incubating,
    Building,
    Validating,
    Operational,
    Paused,
    Archived,
}

impl Lifecycle {
    pub const ALL: [Lifecycle; 6] = [
        Lifecycle::Incubating,
        Lifecycle::Building,
        Lifecycle::Validating,
        Lifecycle::Operational,
        Lifecycle::Paused,
        Lifecycle::Archived,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Lifecycle::Incubating => "incubating",
            Lifecycle::Building => "building",
            Lifecycle::Validating => "validating",
            Lifecycle::Operational => "operational",
            Lifecycle::Paused => "paused",
            Lifecycle::Archived => "archived",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| format!("unknown lifecycle `{raw}`"))
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- confidence --------------------------------------------------------

/// How confident the operator is in the current classification.
/// Independent of the maturity ladder: a project may be `high`
/// confidence while its gate is `unavailable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Unknown,
    Low,
    Medium,
    High,
}

impl Confidence {
    pub const ALL: [Confidence; 4] = [
        Confidence::Unknown,
        Confidence::Low,
        Confidence::Medium,
        Confidence::High,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Confidence::Unknown => "unknown",
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| format!("unknown confidence `{raw}`"))
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- relations ---------------------------------------------------------

/// How two projects relate. The vocabulary is closed: a future
/// relation type is an additive enum variant, never a free-form
/// string, so a read model can always render a known shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationType {
    DependsOn,
    DuplicateOf,
    SharesDomainWith,
    Replaces,
    Consumes,
    OptionalProvider,
}

impl RelationType {
    pub const ALL: [RelationType; 6] = [
        RelationType::DependsOn,
        RelationType::DuplicateOf,
        RelationType::SharesDomainWith,
        RelationType::Replaces,
        RelationType::Consumes,
        RelationType::OptionalProvider,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            RelationType::DependsOn => "depends-on",
            RelationType::DuplicateOf => "duplicate-of",
            RelationType::SharesDomainWith => "shares-domain-with",
            RelationType::Replaces => "replaces",
            RelationType::Consumes => "consumes",
            RelationType::OptionalProvider => "optional-provider",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown relation type `{raw}`; expected one of {}",
                    Self::vocabulary()
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    pub fn vocabulary() -> String {
        Self::labels().join(", ")
    }

    /// No relation type currently permits a self-link, so a
    /// project pointing at itself is always a validation error
    /// rather than a stored no-op.
    pub fn permits_self_relation(&self) -> bool {
        false
    }
}

// --- evidence status ---------------------------------------------------

/// State of one imported source-owned snapshot. Only `observed`
/// claims that somebody actually saw a result; every other value
/// is an honest absence or failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStatus {
    /// The source ran and reported this observation.
    Observed,
    /// The observation exists but is past its freshness bound.
    Stale,
    /// The source could not be reached or is not configured.
    Unavailable,
    /// The source answered, but the answer could not be parsed.
    Invalid,
    /// The source is configured but has never run here.
    NotRun,
}

impl EvidenceStatus {
    pub const ALL: [EvidenceStatus; 5] = [
        EvidenceStatus::Observed,
        EvidenceStatus::Stale,
        EvidenceStatus::Unavailable,
        EvidenceStatus::Invalid,
        EvidenceStatus::NotRun,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            EvidenceStatus::Observed => "observed",
            EvidenceStatus::Stale => "stale",
            EvidenceStatus::Unavailable => "unavailable",
            EvidenceStatus::Invalid => "invalid",
            EvidenceStatus::NotRun => "not-run",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown evidence status `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    /// Whether the state reports a result someone actually
    /// observed. `stale`, `unavailable`, `invalid` and `not-run`
    /// are all absences: none of them may render as healthy or
    /// passing, and none of them may be silently upgraded.
    pub fn reports_observation(&self) -> bool {
        matches!(self, EvidenceStatus::Observed)
    }
}
