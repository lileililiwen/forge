//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

/// One maturity control from the versioned policy descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaturityControl {
    pub id: String,
    pub level: String,
    pub description: String,
    pub applicable: bool,
    pub met: bool,
    pub evidence: Vec<String>,
}
/// Full doctor outcome for one project directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub path: String,
    pub profile: Option<String>,
    pub policy_version: String,
    pub current_maturity: Option<String>,
    pub target_maturity: Option<String>,
    pub findings: Vec<Finding>,
    pub controls: Vec<MaturityControl>,
    pub healthy: bool,
    pub stale: bool,
}
impl DoctorReport {
    /// Findings that deny health: every `FAIL`/`UNAVAILABLE`.
    pub fn blocking_findings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Fail | FindingStatus::Unavailable))
            .collect()
    }
    /// Applicable controls without satisfying evidence.
    pub fn unmet_controls(&self) -> Vec<&MaturityControl> {
        self.controls
            .iter()
            .filter(|c| c.applicable && !c.met)
            .collect()
    }
}
/// Git remote probe via argument arrays (never shell). A repository without
/// an `origin` remote is `missing`; a non-repository is `unknown`.
pub(super) enum GitState {
    Detected(String),
    Missing,
    Unknown,
}
/// One typed doctor finding with a stable rule ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub id: String,
    pub status: FindingStatus,
    pub evidence: Vec<String>,
    pub applicable: bool,
    pub remediation: Remediation,
    pub detail: String,
}
impl Finding {
    pub(super) fn new(
        id: &str,
        status: FindingStatus,
        evidence: Vec<String>,
        applicable: bool,
        remediation: Remediation,
        detail: impl Into<String>,
    ) -> Self {
        Finding {
            id: id.to_string(),
            status,
            evidence,
            applicable,
            remediation,
            detail: detail.into(),
        }
    }
}
/// Outcome of one inspection rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingStatus {
    Pass,
    Warn,
    Fail,
    Unavailable,
}
pub(super) struct Rollup {
    pub(super) status: FindingStatus,
    pub(super) evidence: Vec<String>,
    pub(super) detail: String,
}
/// Read-only registry observation supplied by the caller. `registered`
/// tells whether the project is known; `observed_at` (RFC 3339) is compared
/// against the manifest mtime so stale observations are shown as stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryObservation {
    pub registered: bool,
    pub observed_at: Option<String>,
}
/// Who (or what) can apply the fix for a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Remediation {
    Automatic,
    Ai,
    Manual,
}

/// Version of the maturity policy descriptors compiled into this release.
pub const DOCTOR_POLICY_VERSION: &str = "0.1.0";

impl std::fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingStatus::Pass => write!(f, "PASS"),
            FindingStatus::Warn => write!(f, "WARN"),
            FindingStatus::Fail => write!(f, "FAIL"),
            FindingStatus::Unavailable => write!(f, "UNAVAILABLE"),
        }
    }
}

impl std::fmt::Display for Remediation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Remediation::Automatic => write!(f, "automatic"),
            Remediation::Ai => write!(f, "ai"),
            Remediation::Manual => write!(f, "manual"),
        }
    }
}
