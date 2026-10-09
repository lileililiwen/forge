//! Delivery and studio commands (`delivery`/`studio`).
//!
//! Typed CLI handlers for the staged delivery workflow and the site
//! studio surface. Bodies moved verbatim from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use std::path::{Path, PathBuf};

use super::commands::{DeliveryCommands, StudioCommands, StudioPreviewAction};
use super::feature::{cmd_studio_refine, preview_output};
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_delivery(
    db_path: &Path,
    command: &DeliveryCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DeliveryCommands::Status { project } => cmd_delivery_status(db_path, project, format),
        DeliveryCommands::Preflight { project } => cmd_delivery_preflight(db_path, project, format),
        DeliveryCommands::Stage {
            project,
            confirm_operation_id,
        } => {
            let Some(op_id) = *confirm_operation_id else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery stage: --confirm-operation-id is required".to_string(),
                });
            };
            cmd_delivery_stage(db_path, project, op_id, format)
        }
        DeliveryCommands::Promote {
            project,
            confirm_revision,
        } => {
            let Some(rev) = confirm_revision.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery promote: --confirm-revision is required".to_string(),
                });
            };
            cmd_delivery_promote(db_path, project, rev, format)
        }
        DeliveryCommands::HermoraRetry {
            project,
            deployment_url,
            secret_ref,
        } => {
            let Some(url) = deployment_url.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery hermora-retry: --deployment-url is required".to_string(),
                });
            };
            let Some(ref_name) = secret_ref.as_deref() else {
                return Err(ForgeError::DeliveryInvalid {
                    reason: "delivery hermora-retry: --secret-ref is required".to_string(),
                });
            };
            cmd_delivery_hermora_retry(db_path, project, url, ref_name, format)
        }
    }
}

fn cmd_delivery_status(
    db_path: &Path,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let (report, json) = forge::delivery::cli::cmd_delivery_status(db_path, project)?;
    let human = forge::delivery::projection::render_report_human(&report);
    Ok(as_output(format, human, json))
}

fn cmd_delivery_preflight(
    db_path: &Path,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_preflight(db_path, project)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery preflight: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery preflight recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_stage(
    db_path: &Path,
    project: &str,
    confirm_operation_id: i64,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_stage(db_path, project, confirm_operation_id)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery stage: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery stage recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_promote(
    db_path: &Path,
    project: &str,
    confirm_revision: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_promote(db_path, project, confirm_revision)?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery promote: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nprovider_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery promote recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

fn cmd_delivery_hermora_retry(
    db_path: &Path,
    project: &str,
    deployment_url: &str,
    secret_ref: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let outcome = forge::delivery::cli::cmd_delivery_hermora_retry(
        db_path,
        project,
        deployment_url,
        secret_ref,
    )?;
    let json =
        serde_json::to_value(&outcome.report).map_err(|err| ForgeError::DeliveryInvalid {
            reason: format!("delivery hermora-retry: cannot encode report: {err}"),
        })?;
    let provider_line = outcome
        .provider_status
        .as_deref()
        .map(|s| format!("\nhermora_status: {s}"))
        .unwrap_or_default();
    let human = format!(
        "delivery hermora-retry recorded for `{project}`\nop_id: {}\nphase: {}{provider_line}",
        outcome.op_id, outcome.report.phase
    );
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_studio(
    db_path: &Path,
    command: &StudioCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        StudioCommands::Spec {
            project,
            from,
            expected_revision,
            confirm,
        } => cmd_studio_spec(
            db_path,
            project,
            from.as_deref(),
            expected_revision.as_deref(),
            confirm.as_deref(),
            format,
        ),
        StudioCommands::Preview {
            project,
            action,
            confirm,
        } => cmd_studio_preview(db_path, project, *action, confirm.as_deref(), format),
        StudioCommands::Refine {
            project,
            expected_revision,
            request,
            selected_files,
        } => cmd_studio_refine(
            db_path,
            project,
            expected_revision,
            request,
            selected_files,
            format,
        ),
    }
}

fn cmd_studio_spec(
    db_path: &Path,
    project: &str,
    from: Option<&Path>,
    expected_revision: Option<&str>,
    confirm: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = forge::registry::Registry::open(db_path)?;
    let record = registry.inspect(project)?;
    let path = match from {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from(&record.path).join("forge.app.yaml"),
    };
    let spec = forge::studio::parse_spec_file(&path).map_err(|err| {
        // Add the file path to the message so the operator can
        // locate the broken proposal without a second pass.
        ForgeError::StudioInvalidSpec {
            reason: format!("{err} (path {})", path.display()),
        }
    })?;
    if let Some(token) = confirm {
        if token != "yes" {
            return Err(ForgeError::StudioInvalidSpec {
                reason: "studio spec save requires --confirm yes".to_string(),
            });
        }
        let expected = expected_revision.ok_or_else(|| ForgeError::StudioInvalidSpec {
            reason: "studio spec save requires --expected-revision (use r0 for the first save)"
                .to_string(),
        })?;
        let project_root = PathBuf::from(&record.path);
        let session = forge::studio::save_spec(&registry, project, &project_root, spec, expected)?;
        let envelope = serde_json::json!({
            "contract": forge::studio::STUDIO_SESSION_CONTRACT,
            "project_id": session.project_id,
            "spec_revision": session.spec_revision,
            "app_revision": session.app_revision,
        });
        let human = format!(
            "studio spec saved\nproject_id={}\nspec_revision={}\napp_revision={}",
            session.project_id, session.spec_revision, session.app_revision
        );
        return Ok(as_output(format, human, envelope));
    }
    let envelope = serde_json::json!({
        "contract": forge::studio::APP_SPEC_CONTRACT,
        "project_id": spec.project_id,
        "name": spec.name,
        "profile": spec.profile,
        "schema_version": spec.schema_version,
        "pages": spec.pages,
        "theme": spec.theme,
        "acceptance_checks": spec.acceptance_checks,
    });
    let human = format!(
        "studio spec ok\nproject_id={}\nprofile={}\npages={}\nschema_version={}",
        spec.project_id,
        spec.profile,
        spec.pages.len(),
        spec.schema_version
    );
    Ok(as_output(format, human, envelope))
}

fn cmd_studio_preview(
    db_path: &Path,
    project: &str,
    action: StudioPreviewAction,
    confirm: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = forge::registry::Registry::open(db_path)?;
    let record = registry.inspect(project)?;
    let project_root = PathBuf::from(&record.path);
    let session = forge::studio::load_session(&project_root)?;
    let envelope = match session.as_ref() {
        Some(session) => forge::studio::envelope_from_session(session),
        None => forge::studio::PreviewEnvelope::from_session(
            project,
            "r0",
            &forge::studio::state::SessionPreviewState::default(),
        ),
    };
    match action {
        StudioPreviewAction::Status => Ok(preview_output(&envelope, format)),
        StudioPreviewAction::Start => {
            if confirm != Some("yes") {
                return Err(ForgeError::StudioInvalidSpec {
                    reason: "studio preview start requires --confirm yes".to_string(),
                });
            }
            // Bounded readiness probe: the CLI exits immediately, so
            // it starts the profile runner, returns the `ready`
            // envelope captured at readiness, then tears its own child
            // down. The long-lived API host owns a live session
            // instead (see `ApiConfig`). No detached process survives.
            let runner = Box::new(forge::studio::ProcessRunner::react_web());
            let (session, live) = forge::studio::start_preview(&registry, &project_root, runner)?;
            let ready = forge::studio::envelope_from_session(&session);
            forge::studio::stop_preview(&registry, &project_root, Some(live))?;
            Ok(preview_output(&ready, format))
        }
        StudioPreviewAction::Stop => {
            if confirm != Some("yes") {
                return Err(ForgeError::StudioInvalidSpec {
                    reason: "studio preview stop requires --confirm yes".to_string(),
                });
            }
            let session = forge::studio::stop_preview(&registry, &project_root, None)?;
            let envelope = forge::studio::envelope_from_session(&session);
            Ok(preview_output(&envelope, format))
        }
    }
}
