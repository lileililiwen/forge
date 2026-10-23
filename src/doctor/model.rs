//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

use crate::delivery::state::DeliveryPhase;
use crate::delivery::DeliveryEnvironment;

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
    /// The report's own health partition, judged by the doctor's blocking
    /// set.
    ///
    /// This is the one rule both health projections read: the workbench
    /// health document and the project-status doctor sub-check. A report
    /// that carries blocking evidence is [`HealthPartition::Issues`]; a
    /// report that carries none but whose observation is stale is
    /// [`HealthPartition::Stale`]; a report that carries none and is
    /// current is [`HealthPartition::Healthy`].
    pub fn health_partition(&self) -> HealthPartition {
        if !self.blocking_findings().is_empty() || !self.unmet_controls().is_empty() {
            HealthPartition::Issues
        } else if self.stale {
            HealthPartition::Stale
        } else {
            HealthPartition::Healthy
        }
    }
}

/// How the doctor judges its own report.
///
/// The doctor owns this partition because it is the surface that judges
/// the report; every consumer of a `DoctorReport` reads it instead of
/// re-deriving the verdict from the report's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthPartition {
    /// The report carries blocking evidence: a failing or unavailable
    /// finding, or an unmet maturity control.
    Issues,
    /// The report carries no blocking evidence and its observation is
    /// stale.
    Stale,
    /// The report carries no blocking evidence and its observation is
    /// current.
    Healthy,
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
/// `delivery` carries what the caller could observe about the project's
/// delivery state; `None` means the caller did not look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryObservation {
    pub registered: bool,
    pub observed_at: Option<String>,
    pub delivery: Option<DeliveryObservation>,
}

/// What the caller could observe about the project's delivery state.
///
/// The delivery lane owns the phase vocabulary; the doctor reads it
/// and never computes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryObservation {
    /// The delivery lane recorded a phase for the project.
    Recorded {
        phase: DeliveryPhase,
        environment: Option<DeliveryEnvironment>,
        revision: Option<String>,
        updated_at: String,
    },
    /// The project has no recorded delivery.
    Absent,
    /// The delivery state could not be read.
    Unreadable { reason: String },
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A doctor report with the given findings, controls and staleness.
    fn report(findings: Vec<Finding>, controls: Vec<MaturityControl>, stale: bool) -> DoctorReport {
        DoctorReport {
            path: "/tmp/project".to_string(),
            profile: Some("rust-web".to_string()),
            policy_version: DOCTOR_POLICY_VERSION.to_string(),
            current_maturity: Some("L1".to_string()),
            target_maturity: Some("L1".to_string()),
            findings,
            controls,
            healthy: false,
            stale,
        }
    }

    fn finding(status: FindingStatus) -> Finding {
        Finding::new(
            "rule",
            status,
            vec!["evidence".to_string()],
            true,
            Remediation::Manual,
            "detail",
        )
    }

    /// A control that is applicable, met or not.
    fn control(met: bool) -> MaturityControl {
        MaturityControl {
            id: "L1-structure".to_string(),
            level: "L1".to_string(),
            description: "the project carries its manifest".to_string(),
            applicable: true,
            met,
            evidence: Vec::new(),
        }
    }

    /// The doctor's `healthy` field, expressed the way the runner expressed
    /// it before the partition existed: no `FAIL`/`UNAVAILABLE` finding, no
    /// unmet control and a current observation. The partition must agree
    /// with it for every report, so the field keeps its value.
    fn runner_healthy(findings: &[Finding], controls: &[MaturityControl], stale: bool) -> bool {
        findings
            .iter()
            .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
            && controls.iter().all(|c| !c.applicable || c.met)
            && !stale
    }

    #[test]
    fn a_failing_finding_is_issues() {
        let report = report(vec![finding(FindingStatus::Fail)], Vec::new(), false);
        assert_eq!(report.health_partition(), HealthPartition::Issues);
    }

    #[test]
    fn an_unavailable_finding_is_issues() {
        let report = report(vec![finding(FindingStatus::Unavailable)], Vec::new(), false);
        assert_eq!(report.health_partition(), HealthPartition::Issues);
    }

    #[test]
    fn an_unmet_control_is_issues() {
        let report = report(Vec::new(), vec![control(false)], false);
        assert_eq!(report.health_partition(), HealthPartition::Issues);
    }

    #[test]
    fn a_stale_report_without_blocking_evidence_is_stale() {
        let report = report(
            vec![finding(FindingStatus::Pass), finding(FindingStatus::Warn)],
            vec![control(true)],
            true,
        );
        assert_eq!(report.health_partition(), HealthPartition::Stale);
    }

    #[test]
    fn a_clean_current_report_is_healthy() {
        let report = report(
            vec![finding(FindingStatus::Pass), finding(FindingStatus::Warn)],
            vec![control(true)],
            false,
        );
        assert_eq!(report.health_partition(), HealthPartition::Healthy);
    }

    #[test]
    fn blocking_evidence_outranks_a_stale_observation() {
        let report = report(vec![finding(FindingStatus::Fail)], Vec::new(), true);
        assert_eq!(report.health_partition(), HealthPartition::Issues);
    }

    #[test]
    fn the_healthy_partition_is_the_doctors_own_healthy_field() {
        let shapes: Vec<(Vec<Finding>, Vec<MaturityControl>, bool)> = vec![
            (Vec::new(), Vec::new(), false),
            (Vec::new(), Vec::new(), true),
            (vec![finding(FindingStatus::Pass)], Vec::new(), false),
            (vec![finding(FindingStatus::Warn)], Vec::new(), false),
            (vec![finding(FindingStatus::Fail)], Vec::new(), false),
            (vec![finding(FindingStatus::Unavailable)], Vec::new(), false),
            (Vec::new(), vec![control(true)], false),
            (Vec::new(), vec![control(false)], false),
            (
                vec![finding(FindingStatus::Warn)],
                vec![control(true)],
                true,
            ),
            (
                vec![finding(FindingStatus::Fail)],
                vec![control(false)],
                true,
            ),
        ];
        for (findings, controls, stale) in shapes {
            let mut report = report(findings.clone(), controls.clone(), stale);
            let expected = runner_healthy(&findings, &controls, stale);
            report.healthy = expected;
            assert_eq!(
                matches!(report.health_partition(), HealthPartition::Healthy),
                report.healthy,
                "the healthy partition and the healthy field must agree: {report:?}"
            );
            assert_eq!(
                matches!(report.health_partition(), HealthPartition::Healthy),
                expected,
                "the partition must not change the doctor's own verdict: {report:?}"
            );
        }
    }
}
