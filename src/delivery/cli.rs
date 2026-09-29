//! CLI subcommand dispatchers for `forge delivery *`.

use std::path::Path;

use chrono::Utc;

use crate::core::ForgeError;
use crate::registry::Registry;

use super::handlers::{self, DeliveryOutcome};
use super::projection::DeliveryReport;

/// Run `forge delivery status <project>`.
pub fn cmd_delivery_status(
    db_path: &Path,
    project: &str,
) -> Result<(DeliveryReport, serde_json::Value), ForgeError> {
    let registry = Registry::open(db_path)?;
    let now = Utc::now();
    let report = handlers::run_status(&registry, project, now)?;
    let json = serde_json::to_value(&report).map_err(|err| ForgeError::DeliveryInvalid {
        reason: format!("delivery status: cannot encode report: {err}"),
    })?;
    Ok((report, json))
}

/// Run `forge delivery preflight <project>`.
pub fn cmd_delivery_preflight(
    db_path: &Path,
    project: &str,
) -> Result<DeliveryOutcome, ForgeError> {
    let registry = Registry::open(db_path)?;
    let now = Utc::now();
    handlers::run_preflight(&registry, project, now)
}

/// Run `forge delivery stage <project> --confirm-operation-id <op_id>`.
pub fn cmd_delivery_stage(
    db_path: &Path,
    project: &str,
    confirm_operation_id: i64,
) -> Result<DeliveryOutcome, ForgeError> {
    let registry = Registry::open(db_path)?;
    let now = Utc::now();
    handlers::run_stage(&registry, project, confirm_operation_id, now)
}

/// Run `forge delivery promote <project> --confirm-revision <revision>`.
pub fn cmd_delivery_promote(
    db_path: &Path,
    project: &str,
    confirm_revision: &str,
) -> Result<DeliveryOutcome, ForgeError> {
    let registry = Registry::open(db_path)?;
    let now = Utc::now();
    handlers::run_promote(&registry, project, confirm_revision, now)
}

/// Run `forge delivery hermora-retry <project> --deployment-url <url>
/// --secret-ref <ref>`.
pub fn cmd_delivery_hermora_retry(
    db_path: &Path,
    project: &str,
    deployment_url: &str,
    secret_ref: &str,
) -> Result<DeliveryOutcome, ForgeError> {
    let registry = Registry::open(db_path)?;
    let now = Utc::now();
    handlers::run_hermora_retry(&registry, project, deployment_url, secret_ref, now)
}
