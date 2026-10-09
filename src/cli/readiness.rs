//! Readiness helpers (`readiness`).
//!
//! Readiness matrix/artifact/check handlers shared by the readiness surface.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::readiness::{
    artifact_evidence, evaluate_gate, render_artifact_human, render_gate_human,
    render_matrix_human, run_matrix, READINESS_CONTRACT_VERSION,
};

use super::projects::as_output;
use crate::{Format, Output};

/// honest answer to "which project is more interesting" is a set of
/// labelled figures, and Forge will not manufacture a ranking the
/// windows do not support.
pub(super) fn render_comparison(comparison: &forge::portfolio::interest::Comparison) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "comparable across windows: {}\nrows: {}\n",
        comparison.comparable,
        comparison.rows.len()
    ));
    for window in &comparison.windows {
        out.push_str(&format!("window: {window}\n"));
    }
    if comparison.rows.is_empty() {
        out.push_str("figures: none\n");
    } else {
        out.push_str("figures:\n");
        for row in &comparison.rows {
            out.push_str(&format!(
                "  {} {} = {} [{} / {} / {} / {}]\n",
                row.project_id,
                row.metric,
                row.value,
                row.source,
                row.source_revision,
                row.privacy_mode,
                row.freshness
            ));
        }
    }
    if !comparison.notes.is_empty() {
        out.push_str("notes:\n");
        for note in &comparison.notes {
            out.push_str(&format!("  {note}\n"));
        }
    }
    out
}

pub(super) fn cmd_readiness_matrix(
    profiles: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    // The matrix touches no registry and invents no project: every fixture
    // lives in a disposable directory and the journal stays independent of
    // the readiness surface.
    let report = run_matrix(profiles)?;
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "matrix": report,
    });
    let human = render_matrix_human(&report);
    Ok(as_output(format, human, json))
}

pub(super) fn cmd_readiness_artifact(format: Format) -> Result<Output, ForgeError> {
    let evidence = artifact_evidence()?;
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "artifact": evidence,
    });
    let human = render_artifact_human(&evidence);
    Ok(as_output(format, human, json))
}

pub(super) fn cmd_readiness_check(
    profiles: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    // The gate owns its exit code: the report always renders on stdout so a
    // blocked gate stays observable, while the typed error renders on stderr.
    let (report, gate) = evaluate_gate(profiles);
    let json = serde_json::json!({
        "contract": READINESS_CONTRACT_VERSION,
        "gate": report,
    });
    let human = render_gate_human(&report);
    match gate {
        Ok(()) => Ok(as_output(format, human, json)),
        Err(err) => {
            println!(
                "{}",
                match format {
                    Format::Human | Format::Table => human,
                    Format::Json => serde_json::to_string_pretty(&json).unwrap(),
                    Format::Ndjson => serde_json::to_string(&json).unwrap(),
                }
            );
            Err(err)
        }
    }
}
