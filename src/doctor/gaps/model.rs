//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog::record::SourceKind;
use crate::doctor::Remediation;
use serde::{Deserialize, Serialize};

/// Closed set of gap verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapStatus {
    /// The gap is filled by the available evidence.
    #[serde(rename = "pass")]
    Pass,
    /// The value is present but stale, partial or otherwise non-current.
    #[serde(rename = "warn")]
    Warn,
    /// The gap is open: the value is missing or invalid.
    #[serde(rename = "fail")]
    Fail,
    /// The source cannot be read.
    #[serde(rename = "unavailable")]
    Unavailable,
    /// The control does not apply to the record's source or profile.
    #[serde(rename = "not_applicable")]
    NotApplicable,
}
impl GapStatus {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            GapStatus::Pass => "pass",
            GapStatus::Warn => "warn",
            GapStatus::Fail => "fail",
            GapStatus::Unavailable => "unavailable",
            GapStatus::NotApplicable => "not_applicable",
        }
    }
    /// Parse a `--status` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pass" => Some(GapStatus::Pass),
            "warn" => Some(GapStatus::Warn),
            "fail" => Some(GapStatus::Fail),
            "unavailable" => Some(GapStatus::Unavailable),
            "not_applicable" | "not-applicable" | "na" => Some(GapStatus::NotApplicable),
            _ => None,
        }
    }
    /// Whether this verdict denies health.
    pub fn blocks_health(self) -> bool {
        matches!(self, GapStatus::Fail | GapStatus::Unavailable)
    }
}
/// Full evidence-gap report over a slice of records. Ordering is stable
/// by `(project_id, source, category, subject)` so two readers of the
/// same catalog produce identical bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GapReport {
    pub contract: String,
    pub project_id: Option<String>,
    pub findings: Vec<GapFinding>,
}
impl GapReport {
    /// True when every applicable (non-`not_applicable`) finding passes.
    pub fn clean(&self) -> bool {
        self.findings
            .iter()
            .all(|f| matches!(f.status, GapStatus::Pass | GapStatus::NotApplicable))
    }
}
/// Compose the closed category, status and remediation-class filters
/// used by the CLI. An empty filter accepts everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GapFilters {
    pub categories: Vec<GapCategory>,
    pub statuses: Vec<GapStatus>,
    pub remediation_classes: Vec<RemediationClass>,
}
impl GapFilters {
    /// True when the finding passes every active filter.
    pub fn matches(&self, finding: &GapFinding) -> bool {
        if !self.categories.is_empty() && !self.categories.contains(&finding.category) {
            return false;
        }
        if !self.statuses.is_empty() && !self.statuses.contains(&finding.status) {
            return false;
        }
        if !self.remediation_classes.is_empty()
            && !self
                .remediation_classes
                .contains(&finding.remediation_class)
        {
            return false;
        }
        true
    }
}
/// One typed evidence-gap finding for one record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GapFinding {
    /// Stable id derived from `(category, project_id, subject)`.
    pub id: String,
    /// Verdict.
    pub status: GapStatus,
    /// Closed category.
    pub category: GapCategory,
    /// Subject within the category (e.g. `name`, `tags`, `ci`).
    pub subject: String,
    /// Remediation class (spec vocabulary).
    pub remediation_class: RemediationClass,
    /// Project id the finding is attributed to.
    pub project_id: String,
    /// Source label the record came from (`local`, `workspace-registry`,
    /// `inventory`, `git:<path>`).
    pub source: String,
    /// Source kind tag from the catalog record.
    pub source_kind: SourceKind,
    /// Source revision (the catalog's `source_revision`), if present.
    pub source_revision: Option<String>,
    /// Observation timestamp (the catalog's `observed_at`).
    pub observed_at: String,
    /// Whether the underlying record was `fresh` or `stale` at read time.
    pub freshness: crate::catalog::record::Freshness,
    /// Evidence backing the verdict. Redacted via
    /// `policy::redact_credentials`; a credential-shaped value never
    /// appears in the rendered finding.
    pub evidence: Vec<String>,
    /// Human-facing detail (one short clause; never echoes a secret).
    pub detail: String,
}
/// Closed set of gap categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapCategory {
    Description,
    Tags,
    Ci,
    Compose,
    Manifest,
    Docs,
    Repository,
}
impl GapCategory {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            GapCategory::Description => "description",
            GapCategory::Tags => "tags",
            GapCategory::Ci => "ci",
            GapCategory::Compose => "compose",
            GapCategory::Manifest => "manifest",
            GapCategory::Docs => "docs",
            GapCategory::Repository => "repository",
        }
    }
    /// Parse a `--category` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "description" => Some(GapCategory::Description),
            "tags" => Some(GapCategory::Tags),
            "ci" => Some(GapCategory::Ci),
            "compose" => Some(GapCategory::Compose),
            "manifest" => Some(GapCategory::Manifest),
            "docs" => Some(GapCategory::Docs),
            "repository" => Some(GapCategory::Repository),
            _ => None,
        }
    }
    /// Every category, in the order findings are emitted.
    pub fn all() -> [GapCategory; 7] {
        [
            GapCategory::Description,
            GapCategory::Tags,
            GapCategory::Ci,
            GapCategory::Compose,
            GapCategory::Manifest,
            GapCategory::Docs,
            GapCategory::Repository,
        ]
    }
}
/// Spec's `automatic | semantic | manual` vocabulary. Reused rather
/// than invented to share the doctor and spec-routing classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemediationClass {
    /// Deterministic generator or script produces the value.
    Automatic,
    /// An AI/LLM workflow generates the value.
    Semantic,
    /// A human owner produces the value.
    Manual,
}
impl RemediationClass {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            RemediationClass::Automatic => "automatic",
            RemediationClass::Semantic => "semantic",
            RemediationClass::Manual => "manual",
        }
    }
    /// Parse a `--remediation-class` selector. Unknown values are refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "automatic" => Some(RemediationClass::Automatic),
            "semantic" | "ai" => Some(RemediationClass::Semantic),
            "manual" => Some(RemediationClass::Manual),
            _ => None,
        }
    }
    /// Map into the existing doctor [`Remediation`] enum so a spec router
    /// can consume the finding without a second classification table.
    pub fn as_doctor(self) -> Remediation {
        match self {
            RemediationClass::Automatic => Remediation::Automatic,
            RemediationClass::Semantic => Remediation::Ai,
            RemediationClass::Manual => Remediation::Manual,
        }
    }
}
