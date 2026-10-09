//! Provider commands (`provider`).
//!
//! Typed CLI handlers for controlled evidence-provider matrix/run/inspect.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use std::path::Path;

use super::analytics::resolve_analytics_target;
use super::commands_ops::ProviderCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

use forge::provider::{
    inspect as inspect_provider, matrix as provider_matrix,
    parse_provider as parse_evidence_provider,
    render_descriptor_human as render_provider_descriptor_human,
    render_matrix_human as render_provider_matrix_human,
    render_row_human as render_provider_row_human, run_controlled as run_provider_controlled,
    RunOptions as ProviderRunOptions, PROVIDER_CONTRACT_VERSION, PROVIDER_SYNTHETIC_PROJECT,
};

pub(crate) fn cmd_provider(
    db_path: &Path,
    command: &ProviderCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProviderCommands::Matrix { live } => cmd_provider_matrix(db_path, *live, format),
        ProviderCommands::Run {
            provider,
            target,
            live,
            fixture,
            project_ref,
        } => cmd_provider_run(
            db_path,
            provider,
            target.as_deref(),
            *live,
            fixture.as_deref(),
            project_ref,
            format,
        ),
        ProviderCommands::Inspect { provider } => cmd_provider_inspect(db_path, provider, format),
    }
}

pub(super) fn cmd_provider_matrix(
    db_path: &Path,
    live: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    // The matrix is project-agnostic: it invents no project and the
    // journal row carries the synthetic id.
    let report = provider_matrix(live);
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        PROVIDER_SYNTHETIC_PROJECT,
        "done",
        &format!(
            "provider matrix: supported={} unavailable={} not-run={} disabled={} live={}",
            report.supported, report.unavailable, report.not_run, report.disabled, report.live
        ),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "matrix": report,
    });
    let human = render_provider_matrix_human(&report);
    Ok(as_output(format, human, json))
}

pub(super) fn cmd_provider_run(
    db_path: &Path,
    provider: &str,
    target: Option<&str>,
    live: bool,
    fixture: Option<&Path>,
    project_ref: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let id = parse_evidence_provider(provider)?;
    let options = ProviderRunOptions {
        live,
        fixture: fixture.map(Path::to_path_buf),
        allow_unflagged_live: false,
    };
    // Resolve the attribution: a named target contributes its real
    // project id and directory; otherwise the probe runs in a
    // disposable dir under the synthetic project boundary.
    let (project_id, workdir, temp) = match target {
        Some(name) => {
            let (dir, pid) = resolve_analytics_target(db_path, name)?;
            (pid, dir, false)
        }
        None => {
            let mut dir = std::env::temp_dir();
            dir.push(format!(
                "forge-provider-run-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
            std::fs::create_dir_all(&dir).map_err(|err| ForgeError::ProviderInvalid {
                reason: format!("cannot create probe dir: {err}"),
            })?;
            ("evidence-probe".to_string(), dir, true)
        }
    };
    let row = run_provider_controlled(&id, &options, &project_id, &workdir, project_ref)?;
    if temp {
        let _ = std::fs::remove_dir_all(&workdir);
    }
    let journal_project = if target.is_some() {
        project_id.clone()
    } else {
        PROVIDER_SYNTHETIC_PROJECT.to_string()
    };
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        &journal_project,
        if row.status == "supported" {
            "done"
        } else {
            "rejected"
        },
        &format!(
            "provider run {id}: {} (sandbox {})",
            row.status,
            options_sandbox_label(&options)
        ),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "run": row,
    });
    let human = render_provider_row_human(&row);
    Ok(as_output(format, human, json))
}

fn options_sandbox_label(options: &ProviderRunOptions) -> &'static str {
    if options.fixture.is_some() {
        "fixture"
    } else if options.live {
        "live"
    } else {
        "none"
    }
}

pub(super) fn cmd_provider_inspect(
    db_path: &Path,
    provider: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let descriptor = inspect_provider(provider)?;
    let registry = open_registry(db_path)?;
    let _ = registry.record_operation(
        "provider",
        PROVIDER_SYNTHETIC_PROJECT,
        "done",
        &format!("provider inspect {}", descriptor.provider),
    );
    let json = serde_json::json!({
        "contract": PROVIDER_CONTRACT_VERSION,
        "descriptor": descriptor,
    });
    let human = render_provider_descriptor_human(&descriptor);
    Ok(as_output(format, human, json))
}
