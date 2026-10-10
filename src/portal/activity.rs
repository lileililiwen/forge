//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::registry::Registry;
use std::path::Path;

use super::model::{PortalDashboard, PortalOperation};
use super::render::render_section_human;

pub(super) fn count_subdirs(path: &Path) -> usize {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count()
}

pub(super) fn count_files(path: &Path) -> usize {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .count()
}

pub(super) fn count_component_receipts(path: &Path) -> usize {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut total = 0usize;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let receipts = entry.path().join("qualify.json");
            if receipts.exists() {
                total += 1;
            }
        }
    }
    total
}

pub(super) fn count_session_files(path: &Path) -> usize {
    let sessions = path.join("sessions");
    let entries = match std::fs::read_dir(&sessions) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                && entry.file_name().to_string_lossy().ends_with(".json")
        })
        .count()
}

pub(super) fn read_recent_operations(
    registry: &Registry,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<PortalOperation>, ForgeError> {
    let entries = match project_id {
        Some(id) => registry.operations_for_project(id, limit)?,
        None => registry.recent_operations(limit)?,
    };
    Ok(entries
        .into_iter()
        .map(|op| PortalOperation {
            op_id: op.op_id,
            kind: op.kind,
            project_id: op.project_id,
            state: op.state,
            started_at: op.started_at,
            finished_at: op.finished_at,
            detail: op.detail,
        })
        .collect())
}

/// Build a `forge portal dashboard <target>` journal
/// detail line. The line is the human-readable summary
/// the CLI prints on stdout before the typed `error[…]`
/// exit code, so a partial run is observable.
pub fn render_dashboard_human(view: &PortalDashboard) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "forge portal dashboard ({title}) — contract {contract} scope {scope} generated_at {ts}\n",
        title = view.title,
        contract = view.contract,
        scope = view.scope.id(),
        ts = view.generated_at,
    ));
    out.push_str(
        "interactive surface: the SPA dashboard served by `forge web` (routes /projects /workbench /management /portfolio /delivery) — this legacy server-side HTML is a read-only pointer\n",
    );
    if let Some(id) = view.project_id.as_ref() {
        out.push_str(&format!("project_id: {id}\n"));
    }
    let rollup = view.rollup();
    out.push_str(&format!("rollup: {}\n", rollup.id()));
    out.push_str(&format!(
        "sections: {n} ({list})\n",
        n = view.sections.len(),
        list = view
            .sections
            .iter()
            .map(|s| format!("{}=`{}`", s.section_id, s.status.id()))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for section in &view.sections {
        out.push_str(&render_section_human(section));
        out.push('\n');
    }
    if !view.operations.is_empty() {
        out.push_str("recent operations:\n");
        for op in &view.operations {
            let finished = op.finished_at.as_deref().unwrap_or("-");
            out.push_str(&format!(
                "  - op#{id} {kind} {state} project={pid} started={started} finished={finished}\n",
                id = op.op_id,
                kind = op.kind,
                state = op.state,
                pid = op.project_id,
                started = op.started_at,
                finished = finished
            ));
            if let Some(detail) = op.detail.as_ref() {
                out.push_str(&format!("    detail: {detail}\n"));
            }
        }
    } else {
        out.push_str("recent operations: (none)\n");
    }
    out
}
