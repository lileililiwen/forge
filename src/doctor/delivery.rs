//! The doctor's delivery-state finding.
//!
//! One rule over the delivery lane's own phase vocabulary: a healthy
//! delivery passes, a failed delivery fails, a delivery in progress warns,
//! a project with no recorded delivery is not applicable, and a state that
//! cannot be read is unavailable (never a silent pass).
//!
//! Every string that reaches the report passes
//! [`crate::policy::redact_credentials`] first.

use crate::delivery::state::DeliveryPhase;

use super::model::{DeliveryObservation, Finding, FindingStatus, Remediation};

/// The `delivery-state` finding for one observed delivery state.
///
/// `None` means the caller did not look at the delivery: no finding is
/// produced, because the doctor never guesses what it did not read.
pub(super) fn delivery_finding(delivery: Option<&DeliveryObservation>) -> Option<Finding> {
    match delivery? {
        DeliveryObservation::Absent => Some(Finding::new(
            "delivery-state",
            FindingStatus::Pass,
            vec![redact("no delivery has been recorded for this project")],
            false,
            Remediation::Automatic,
            "no delivery has been recorded; the doctor does not decide whether a delivery \
             is required (forge delivery status <project> shows the recorded state)",
        )),
        DeliveryObservation::Unreadable { reason } => Some(Finding::new(
            "delivery-state",
            FindingStatus::Unavailable,
            vec![redact(&format!(
                "delivery state could not be read: {reason}"
            ))],
            false,
            Remediation::Automatic,
            "the delivery state could not be read and is never assumed healthy; run \
             forge delivery status <project> for the evidence",
        )),
        DeliveryObservation::Recorded {
            phase,
            environment,
            revision,
            updated_at,
        } => {
            let mut evidence = vec![redact(&format!("delivery phase={phase}"))];
            if let Some(environment) = environment {
                evidence.push(redact(&format!("environment={environment}")));
            }
            if let Some(revision) = revision {
                evidence.push(redact(&format!("delivered revision={revision}")));
            }
            evidence.push(redact(&format!("delivery state last changed {updated_at}")));
            Some(Finding::new(
                "delivery-state",
                status_for_phase(*phase),
                evidence,
                true,
                Remediation::Manual,
                detail_for_phase(*phase),
            ))
        }
    }
}

/// The closed phase-to-status rule.
///
/// A failed delivery denies health; a delivery in progress is a fact, not a
/// fault, so it warns. `hermora-failed` warns because the delivery lane's
/// own wording is "deployment stays healthy": only the optional enrolment
/// did not complete.
fn status_for_phase(phase: DeliveryPhase) -> FindingStatus {
    match phase {
        DeliveryPhase::Healthy | DeliveryPhase::HermoraConnected => FindingStatus::Pass,
        DeliveryPhase::StageFailed | DeliveryPhase::Degraded => FindingStatus::Fail,
        DeliveryPhase::HermoraFailed => FindingStatus::Warn,
        DeliveryPhase::RolledBack => FindingStatus::Warn,
        DeliveryPhase::Draft
        | DeliveryPhase::Preflighted
        | DeliveryPhase::AwaitingStageConfirmation
        | DeliveryPhase::Staging
        | DeliveryPhase::StageHealthy
        | DeliveryPhase::AwaitingProductionApproval
        | DeliveryPhase::Production
        | DeliveryPhase::HermoraPending => FindingStatus::Warn,
    }
}

/// What the operator does next. Every branch names the command that shows
/// the delivery evidence.
fn detail_for_phase(phase: DeliveryPhase) -> &'static str {
    match phase {
        DeliveryPhase::Healthy | DeliveryPhase::HermoraConnected => {
            "the last delivery is healthy; run forge delivery status <project> for the evidence"
        }
        DeliveryPhase::StageFailed | DeliveryPhase::Degraded => {
            "the last delivery failed; run forge delivery status <project> for the evidence"
        }
        DeliveryPhase::HermoraFailed => {
            "the deployment stays healthy but the optional enrolment did not complete; run \
             forge delivery status <project> for the evidence"
        }
        DeliveryPhase::RolledBack => {
            "production was rolled back to an earlier release and the registered revision is not \
             the live one; run forge delivery status <project> for the restored revision, then \
             fix and promote forward"
        }
        DeliveryPhase::Draft
        | DeliveryPhase::Preflighted
        | DeliveryPhase::AwaitingStageConfirmation
        | DeliveryPhase::Staging
        | DeliveryPhase::StageHealthy
        | DeliveryPhase::AwaitingProductionApproval
        | DeliveryPhase::Production
        | DeliveryPhase::HermoraPending => {
            "the delivery is in progress; run forge delivery status <project> for the evidence"
        }
    }
}

/// Scrub one rendered string before it reaches the report.
fn redact(text: &str) -> String {
    crate::policy::redact_credentials(text)
}
