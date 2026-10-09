//! Analytics inspect helpers (`analytics`).
//!
//! Target resolution and inspect handler for the analytics surface.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::analytics::{
    inspect_external_planes, load_config as load_analytics_config,
    render_report_human as render_analytics_report_human, AnalyticsInspectOptions,
    ANALYTICS_CONTRACT_VERSION,
};
use forge::core::ForgeError;
use std::path::{Path, PathBuf};

use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(super) fn resolve_analytics_target(
    db_path: &Path,
    target: &str,
) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
        return Ok((dir, manifest.project.id));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    Ok((dir, manifest.project.id))
}

pub(super) fn cmd_analytics_inspect(
    db_path: &Path,
    target: &str,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (dir, project_id) = resolve_analytics_target(db_path, target)?;
    let _ = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    let config = match load_analytics_config(&dir)? {
        Some(cfg) => cfg,
        None => {
            let detail = "inspect: project has no analytics block".to_string();
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("analytics", &project_id, "rejected", &detail);
            }
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "project `{project_id}` has no `analytics:` block; declare one in \
                     forge.yaml to enable the external content / analytics plane"
                ),
            });
        }
    };
    let report =
        inspect_external_planes(&project_id, &config, &AnalyticsInspectOptions { dry_run })?;
    let detail = format!(
        "inspect: project={} enabled={} observations={} healthy={} dry_run={}",
        project_id,
        report.enabled,
        report.observations.len(),
        report.healthy(),
        dry_run,
    );
    if let Ok(registry) = open_registry(db_path) {
        let state = if report.healthy() { "done" } else { "rejected" };
        let _ = registry.record_operation("analytics", &project_id, state, &detail);
    }
    let json = serde_json::json!({
        "contract": ANALYTICS_CONTRACT_VERSION,
        "project_id": project_id,
        "report": report,
    });
    let human = render_analytics_report_human(&report);
    Ok(as_output(format, human, json))
}
