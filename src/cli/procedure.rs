//! Procedure commands (`procedure`).
//!
//! Typed CLI handlers for portable AI procedures.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::procedure::{
    inspect_procedure, procedure_catalog, render_inspect_human as render_procedure_inspect_human,
    render_list_human as render_procedure_list_human, validate_procedure, ProcedureSpec,
    PROCEDURE_CONTRACT_VERSION, PROCEDURE_SYNTHETIC_PROJECT,
};
use std::path::Path;

use super::commands::ProcedureCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(crate) fn cmd_procedure(
    db_path: &Path,
    command: &ProcedureCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProcedureCommands::List => {
            let entries = procedure_catalog();
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "procedure",
                    PROCEDURE_SYNTHETIC_PROJECT,
                    "done",
                    &format!("list: {} procedure(s) returned", entries.len()),
                );
            }
            let human = render_procedure_list_human(&entries);
            let json = serde_json::json!({
                "contract": PROCEDURE_CONTRACT_VERSION,
                "procedures": entries,
            });
            Ok(as_output(format, human, json))
        }
        ProcedureCommands::Inspect { id } => match inspect_procedure(id) {
            Ok(spec) => {
                if let Ok(registry) = open_registry(db_path) {
                    let _ = registry.record_operation(
                        "procedure",
                        &spec.id,
                        "done",
                        &format!("inspect: {} step(s)", spec.steps.len()),
                    );
                }
                let human = render_procedure_inspect_human(&spec);
                let json = serde_json::json!({
                    "contract": PROCEDURE_CONTRACT_VERSION,
                    "procedure": spec,
                });
                Ok(as_output(format, human, json))
            }
            Err(err) => {
                if let Ok(registry) = open_registry(db_path) {
                    let _ =
                        registry.record_operation("procedure", id, "rejected", &err.to_string());
                }
                Err(err)
            }
        },
        ProcedureCommands::Validate { path } => {
            let bytes = std::fs::read(path).map_err(|err| ForgeError::ProcedureInvalid {
                reason: format!("could not read procedure spec at {}: {err}", path.display()),
            })?;
            let spec: ProcedureSpec =
                serde_json::from_slice(&bytes).map_err(|err| ForgeError::ProcedureInvalid {
                    reason: format!(
                        "procedure spec at {} is not valid JSON ProcedureSpec: {err}",
                        path.display()
                    ),
                })?;
            let validated = validate_procedure(&spec)?;
            let note = format!(
                "procedure '{}' validated: {} step(s) and ends with report_findings",
                validated.id,
                validated.steps.len()
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation("procedure", &validated.id, "done", &note);
            }
            let json = serde_json::json!({
                "contract": PROCEDURE_CONTRACT_VERSION,
                "procedure": validated,
                "note": note,
            });
            Ok(as_output(
                format,
                render_procedure_inspect_human(&validated),
                json,
            ))
        }
    }
}
