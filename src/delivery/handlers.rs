//! Delivery verb orchestration.
//!
//! Each handler is a thin adapter: validate arguments, derive the
//! deterministic idempotency key, reserve / finalize the journal
//! row, invoke the publish provider (or the Hermora adapter), and
//! record the resulting evidence. The handlers share the same
//! `Registry::finalize_operation` + `Registry::update_operation_phase`
//! shape the existing `publish` and `deploy` flows use.
//!
//! No new schema; the handlers exclusively read and write the
//! existing `operations` table.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::core::ForgeError;
use crate::publish::providers::{
    load_config as load_provider_config, select_provider, ProviderEntry,
};
use crate::registry::{OperationEntry, Registry, ReservationOutcome};

use super::hermora::{self, HermoraOutcome, HermoraRequest};
use super::invoke as provider_invoke;
use super::projection::DeliveryReport;
use super::state::{idempotency_key, request_hash, scrub_detail, DeliveryVerb};
use super::DeliveryEnvironment;

/// Outcome of one delivery verb invocation. The CLI renders the
/// `DeliveryReport`; the API returns the JSON envelope.
#[derive(Debug, Clone)]
pub struct DeliveryOutcome {
    pub op_id: i64,
    pub report: DeliveryReport,
    /// Echoed back from the provider when applicable. `None` when
    /// the verb did not invoke a provider (e.g. hermora retry).
    pub provider_status: Option<String>,
}

const PROVIDER_ID: &str = "openpanel";

/// Resolve the configured publish provider for one project. The
/// provider config is loaded from `$FORGE_PUBLISH_PROVIDER_CONFIG`
/// or `<project>/.forge/providers.yaml`. An unknown provider id
/// surfaces as `delivery-invalid`; a missing config file surfaces
/// as `delivery-unavailable`.
fn resolve_provider(project_dir: &Path) -> Result<ProviderEntry, ForgeError> {
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = load_provider_config(&config_path).map_err(|err| {
        if err.code() == "publish-invalid" {
            ForgeError::DeliveryUnavailable {
                reason: format!(
                    "publish provider config {} is missing or invalid: {}",
                    config_path.display(),
                    err
                ),
            }
        } else {
            ForgeError::DeliveryUnavailable {
                reason: err.to_string(),
            }
        }
    })?;
    select_provider(&config, PROVIDER_ID).map_err(|err| match err.code() {
        "publish-invalid" => ForgeError::DeliveryInvalid {
            reason: format!(
                "publish provider `{PROVIDER_ID}` is not configured in {}: {}",
                config_path.display(),
                err
            ),
        },
        _ => ForgeError::DeliveryUnavailable {
            reason: err.to_string(),
        },
    })
}

/// Locate the project directory and registered source revision.
fn resolve_project(registry: &Registry, project_id: &str) -> Result<(PathBuf, String), ForgeError> {
    let record = registry.inspect(project_id)?;
    let path = PathBuf::from(&record.path);
    if !path.is_dir() {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery: project `{project_id}` source directory `{}` is not available",
                path.display()
            ),
        });
    }
    let revision = record
        .last_commit
        .clone()
        .filter(|rev| rev.len() == 40 && rev.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery: project `{project_id}` has no 40-character source revision recorded; \
                 re-run `forge register` or commit and re-register before delivery"
            ),
        })?;
    Ok((path, revision))
}

/// Build the projection after a single mutation by re-reading the
/// journal. The `Registry` methods we call are all `&self` so
/// multiple immutable borrows are safe.
fn project(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryReport, ForgeError> {
    super::projection::build_delivery_report(registry, project_id, now)
}

/// Run `forge delivery preflight <project>`.
pub fn run_preflight(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryOutcome, ForgeError> {
    let (project_dir, revision) = resolve_project(registry, project_id)?;
    let provider = resolve_provider(&project_dir)?;
    let key = idempotency_key(DeliveryVerb::Preflight, project_id, &revision)?;
    let hash = request_hash(&revision, None, "preflight");
    let reservation =
        registry.reserve_idempotent_operation("delivery.preflight", project_id, &key, &hash)?;
    let op_id = match reservation {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            let report = project(registry, project_id, now)?;
            return Ok(DeliveryOutcome {
                op_id,
                report,
                provider_status: None,
            });
        }
    };

    let request = provider_invoke::build_request(
        DeliveryVerb::Preflight,
        &provider,
        project_id,
        &revision,
        &project_dir,
        &format!("delivery-preflight-{op_id}"),
    );
    let response = match provider_invoke::invoke(&provider, &request, &project_dir) {
        Ok(response) => response,
        Err(err) => {
            let _ = registry.finalize_operation(op_id, "failed", &scrub_detail(&err.to_string()));
            return Err(err);
        }
    };
    let detail = encode_provider_detail(&response);
    let state = if response.status == "succeeded" {
        "done"
    } else {
        "failed"
    };
    registry.finalize_operation(op_id, state, &scrub_detail(&detail))?;
    let _ = registry.update_operation_phase(
        op_id,
        Some(&revision),
        response.build_status.as_deref(),
        response.run_status.as_deref(),
        response.container_identity.as_deref(),
    );
    let report = project(registry, project_id, now)?;
    Ok(DeliveryOutcome {
        op_id,
        report,
        provider_status: Some(response.status),
    })
}

/// Run `forge delivery stage <project> --confirm-operation-id <op_id>`.
pub fn run_stage(
    registry: &Registry,
    project_id: &str,
    confirm_operation_id: i64,
    now: DateTime<Utc>,
) -> Result<DeliveryOutcome, ForgeError> {
    let (project_dir, revision) = resolve_project(registry, project_id)?;
    let provider = resolve_provider(&project_dir)?;
    verify_preflight_for_revision(registry, project_id, confirm_operation_id, &revision)?;

    let key = idempotency_key(DeliveryVerb::Stage, project_id, &revision)?;
    let hash = request_hash(
        &revision,
        Some(DeliveryEnvironment::Stage),
        &confirm_operation_id.to_string(),
    );
    let reservation =
        registry.reserve_idempotent_operation("delivery.stage", project_id, &key, &hash)?;
    let op_id = match reservation {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            let report = project(registry, project_id, now)?;
            return Ok(DeliveryOutcome {
                op_id,
                report,
                provider_status: None,
            });
        }
    };

    let request = provider_invoke::build_request(
        DeliveryVerb::Stage,
        &provider,
        project_id,
        &revision,
        &project_dir,
        &format!("delivery-stage-{op_id}"),
    );
    let response = match provider_invoke::invoke(&provider, &request, &project_dir) {
        Ok(response) => response,
        Err(err) => {
            let _ = registry.finalize_operation(op_id, "failed", &scrub_detail(&err.to_string()));
            return Err(err);
        }
    };
    let detail = encode_provider_detail(&response);
    let state = if response.status == "succeeded" {
        "done"
    } else {
        "failed"
    };
    registry.finalize_operation(op_id, state, &scrub_detail(&detail))?;
    let _ = registry.update_operation_phase(
        op_id,
        Some(&revision),
        response.build_status.as_deref(),
        response.run_status.as_deref(),
        response.container_identity.as_deref(),
    );
    let report = project(registry, project_id, now)?;
    Ok(DeliveryOutcome {
        op_id,
        report,
        provider_status: Some(response.status),
    })
}

/// Run `forge delivery promote <project> --confirm-revision <revision>`.
pub fn run_promote(
    registry: &Registry,
    project_id: &str,
    confirm_revision: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryOutcome, ForgeError> {
    if confirm_revision.len() != 40 || !confirm_revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery promote: --confirm-revision `{confirm_revision}` is not a 40-character hex SHA"
            ),
        });
    }
    let (project_dir, registered_revision) = resolve_project(registry, project_id)?;
    if confirm_revision != registered_revision {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery promote: project `{project_id}` --confirm-revision \
                 `{confirm_revision}` does not match the project's registered \
                 source revision `{registered_revision}`; refusing to invoke \
                 the production provider"
            ),
        });
    }
    verify_stage_healthy_for_revision(registry, project_id, &registered_revision)?;
    let provider = resolve_provider(&project_dir)?;

    let key = idempotency_key(DeliveryVerb::Promote, project_id, &registered_revision)?;
    let hash = request_hash(
        &registered_revision,
        Some(DeliveryEnvironment::Production),
        confirm_revision,
    );
    let reservation =
        registry.reserve_idempotent_operation("delivery.promote", project_id, &key, &hash)?;
    let op_id = match reservation {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            let report = project(registry, project_id, now)?;
            return Ok(DeliveryOutcome {
                op_id,
                report,
                provider_status: None,
            });
        }
    };

    let request = provider_invoke::build_request(
        DeliveryVerb::Promote,
        &provider,
        project_id,
        &registered_revision,
        &project_dir,
        &format!("delivery-promote-{op_id}"),
    );
    let response = match provider_invoke::invoke(&provider, &request, &project_dir) {
        Ok(response) => response,
        Err(err) => {
            let _ = registry.finalize_operation(op_id, "failed", &scrub_detail(&err.to_string()));
            return Err(err);
        }
    };
    let detail = encode_provider_detail(&response);
    let state = if response.status == "succeeded" {
        "done"
    } else {
        "failed"
    };
    registry.finalize_operation(op_id, state, &scrub_detail(&detail))?;
    let _ = registry.update_operation_phase(
        op_id,
        Some(&registered_revision),
        response.build_status.as_deref(),
        response.run_status.as_deref(),
        response.container_identity.as_deref(),
    );
    let report = project(registry, project_id, now)?;
    Ok(DeliveryOutcome {
        op_id,
        report,
        provider_status: Some(response.status),
    })
}

/// Run `forge delivery hermora-retry <project> --deployment-url <url>
/// --secret-ref <ref>`.
pub fn run_hermora_retry(
    registry: &Registry,
    project_id: &str,
    deployment_url: &str,
    secret_ref: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryOutcome, ForgeError> {
    let (project_dir, revision) = resolve_project(registry, project_id)?;
    let _ = project_dir;
    if deployment_url.is_empty() || deployment_url.len() > 2048 {
        return Err(ForgeError::DeliveryInvalid {
            reason: "delivery hermora-retry: --deployment-url must be 1..=2048 chars".to_string(),
        });
    }
    if !deployment_url.starts_with("https://") && !deployment_url.starts_with("http://") {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery hermora-retry: --deployment-url `{deployment_url}` is not an http(s) URL"
            ),
        });
    }
    if secret_ref.is_empty() || secret_ref.len() > 128 {
        return Err(ForgeError::DeliveryInvalid {
            reason: "delivery hermora-retry: --secret-ref must be 1..=128 chars".to_string(),
        });
    }
    if !secret_ref.starts_with("env:") {
        return Err(ForgeError::DeliveryInvalid {
            reason: "delivery hermora-retry: --secret-ref must start with `env:`; \
                     only the environment-variable name is read by the adapter, \
                     never the secret value"
                .to_string(),
        });
    }
    verify_promote_for_revision(registry, project_id, &revision)?;

    let key = idempotency_key(DeliveryVerb::Hermora, project_id, &revision)?;
    let hash = request_hash(
        &revision,
        Some(DeliveryEnvironment::Production),
        &format!("hermora:{deployment_url}:{secret_ref}"),
    );
    let reservation =
        registry.reserve_idempotent_operation("delivery.hermora", project_id, &key, &hash)?;
    let op_id = match reservation {
        ReservationOutcome::Reserved { op_id } => op_id,
        ReservationOutcome::Reused { op_id } => {
            let report = project(registry, project_id, now)?;
            return Ok(DeliveryOutcome {
                op_id,
                report,
                provider_status: None,
            });
        }
    };

    let adapter = match hermora::resolve_adapter(None) {
        Some(path) => path,
        None => {
            let reason = "delivery hermora-retry: `forge-hermora-adapter` is not on PATH \
                          and $FORGE_HERMORA_BIN is unset; deployment stays healthy"
                .to_string();
            let _ = registry.finalize_operation(op_id, "failed", &scrub_detail(&reason));
            return Err(ForgeError::DeliveryUnavailable { reason });
        }
    };

    let request = HermoraRequest {
        contract: hermora::HERMORA_CONTRACT,
        operation: "register",
        project_id: project_id.to_string(),
        environment: DeliveryEnvironment::Production.label().to_string(),
        revision: revision.clone(),
        deployment_url: deployment_url.to_string(),
        secret_ref: secret_ref.to_string(),
    };
    let timeout = hermora::adapter_timeout();
    let outcome = match hermora::invoke(&adapter, &request, timeout) {
        Ok(outcome) => outcome,
        Err(err) => {
            let _ = registry.finalize_operation(op_id, "failed", &scrub_detail(&err.to_string()));
            return Err(err);
        }
    };

    let detail = encode_hermora_detail(&outcome);
    let state = match &outcome {
        HermoraOutcome::Connected { .. } => "done",
        HermoraOutcome::Failed { .. } => "failed",
        HermoraOutcome::Unavailable { .. } => "failed",
    };
    registry.finalize_operation(op_id, state, &scrub_detail(&detail))?;
    if matches!(outcome, HermoraOutcome::Unavailable { .. }) {
        return Err(ForgeError::DeliveryUnavailable {
            reason: match outcome {
                HermoraOutcome::Unavailable { reason } => reason,
                _ => "hermora adapter returned an unavailable outcome".to_string(),
            },
        });
    }
    let report = project(registry, project_id, now)?;
    Ok(DeliveryOutcome {
        op_id,
        report,
        provider_status: Some(outcome.label().to_string()),
    })
}

/// Run `forge delivery status <project>`.
pub fn run_status(
    registry: &Registry,
    project_id: &str,
    now: DateTime<Utc>,
) -> Result<DeliveryReport, ForgeError> {
    project(registry, project_id, now)
}

fn verify_preflight_for_revision(
    registry: &Registry,
    project_id: &str,
    confirm_operation_id: i64,
    expected_revision: &str,
) -> Result<(), ForgeError> {
    let row = registry
        .operation(confirm_operation_id)
        .map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery stage: confirm operation `{confirm_operation_id}` could not be read: {err}"
            ),
        })?
        .ok_or_else(|| ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery stage: confirm operation `{confirm_operation_id}` is not recorded"
            ),
        })?;
    if row.project_id != project_id || row.kind != "delivery.preflight" {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery stage: confirm operation `{confirm_operation_id}` is not a \
                 delivery.preflight row for project `{project_id}`"
            ),
        });
    }
    let row_revision = row.revision.as_deref().unwrap_or("");
    if row_revision != expected_revision {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery stage: confirm operation `{confirm_operation_id}` references \
                 revision `{row_revision}` but the project's registered revision is \
                 `{expected_revision}`; refusing to start stage"
            ),
        });
    }
    if row.state != "done" {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery stage: confirm operation `{confirm_operation_id}` is in state `{}`; \
                 a healthy preflight is required before stage",
                row.state
            ),
        });
    }
    Ok(())
}

fn verify_stage_healthy_for_revision(
    registry: &Registry,
    project_id: &str,
    expected_revision: &str,
) -> Result<(), ForgeError> {
    let rows = registry.operations_for_project(project_id, 50)?;
    let stage = rows
        .into_iter()
        .filter(|row| row.kind == "delivery.stage")
        .max_by_key(|row| row.op_id);
    let Some(row) = stage else {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery promote: project `{project_id}` has no terminal stage row; \
                 refusing to promote without a healthy stage"
            ),
        });
    };
    if row.revision.as_deref() != Some(expected_revision) {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery promote: latest stage row references revision `{}` \
                 but the project's registered revision is `{expected_revision}`",
                row.revision.unwrap_or_default()
            ),
        });
    }
    if row.state != "done" {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery promote: latest stage row is `{}`; refusing to promote a \
                 failing stage",
                row.state
            ),
        });
    }
    if row.build_status.as_deref() == Some("failed") || row.run_status.as_deref() == Some("failed")
    {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery promote: latest stage row reports a failed phase \
                 (build={}, run={}); refusing to promote",
                row.build_status.unwrap_or_else(|| "unknown".to_string()),
                row.run_status.unwrap_or_else(|| "unknown".to_string())
            ),
        });
    }
    Ok(())
}

fn verify_promote_for_revision(
    registry: &Registry,
    project_id: &str,
    expected_revision: &str,
) -> Result<(), ForgeError> {
    let rows = registry.operations_for_project(project_id, 50)?;
    let promote = rows
        .into_iter()
        .filter(|row| row.kind == "delivery.promote")
        .max_by_key(|row| row.op_id);
    let Some(row) = promote else {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery hermora-retry: project `{project_id}` has no terminal promote row; \
                 refusing to call the hermora adapter without a confirmed deployment"
            ),
        });
    };
    if row.revision.as_deref() != Some(expected_revision) {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery hermora-retry: latest promote row references revision `{}` \
                 but the project's registered revision is `{expected_revision}`",
                row.revision.unwrap_or_default()
            ),
        });
    }
    if row.state != "done" {
        return Err(ForgeError::DeliveryConflict {
            reason: format!(
                "delivery hermora-retry: latest promote row is `{}`; refusing to retry \
                 hermora without a healthy deployment",
                row.state
            ),
        });
    }
    Ok(())
}

fn encode_provider_detail(response: &crate::publish::providers::PublishProviderResponse) -> String {
    let mut evidence = response.evidence.clone();
    if evidence.len() > super::projection::EVIDENCE_LIMIT {
        evidence.truncate(super::projection::EVIDENCE_LIMIT);
    }
    let mut recovery = response.recovery.clone();
    if recovery.len() > super::projection::RECOVERY_LIMIT {
        recovery.truncate(super::projection::RECOVERY_LIMIT);
    }
    serde_json::to_string(&json!({
        "evidence": evidence,
        "recovery": recovery,
        "provider_status": response.status,
        "provider_health": response.health,
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn encode_hermora_detail(outcome: &HermoraOutcome) -> String {
    let value = match outcome {
        HermoraOutcome::Connected {
            site_id,
            environment_url,
        } => json!({
            "site_id": site_id,
            "environment_url": environment_url,
            "reason": null,
        }),
        HermoraOutcome::Failed { reason } => json!({
            "site_id": null,
            "environment_url": null,
            "reason": reason,
        }),
        HermoraOutcome::Unavailable { reason } => json!({
            "site_id": null,
            "environment_url": null,
            "reason": reason,
        }),
    };
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
}

#[allow(dead_code)]
fn _ensure_operation_entry_used(entry: OperationEntry) -> OperationEntry {
    entry
}
