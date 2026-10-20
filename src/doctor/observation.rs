//! Read-only delivery observation for the doctor.
//!
//! The delivery lane owns the phase vocabulary. The doctor reads it
//! through the lane's own door — `delivery::projection::build_delivery_report`,
//! the same projection `forge delivery status` and the delivery API
//! answer with — and never computes a phase of its own.
//!
//! Reading is observation, not action: no journal row is written, no
//! registry byte changes, no target file is touched, no provider runs and
//! no delivery verb starts.

use chrono::{DateTime, Utc};

use crate::delivery::state::DeliveryPhase;
use crate::registry::Registry;

use super::model::DeliveryObservation;

/// Read the delivery state the delivery lane recorded for `project_id`.
///
/// A project with no delivery rows reads as `Absent`: the doctor never
/// invents a delivery and never decides that one is required. A state that
/// cannot be read reads as `Unreadable` with the redacted reason, never
/// as a pass.
pub fn delivery_observation(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> DeliveryObservation {
    match crate::delivery::projection::build_delivery_report(registry, project_id, now) {
        Ok(report) => {
            if report.phase == DeliveryPhase::Draft {
                // No delivery verb ever ran for this project: there is
                // no recorded delivery to report.
                DeliveryObservation::Absent
            } else {
                DeliveryObservation::Recorded {
                    phase: report.phase,
                    environment: report.environment,
                    revision: report.revision,
                    updated_at: report.updated_at,
                }
            }
        }
        Err(err) => DeliveryObservation::Unreadable {
            reason: crate::policy::redact_credentials(&err.to_string()),
        },
    }
}
