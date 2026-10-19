//! Local developer workflow facade for Forge-owned modules.

use std::path::PathBuf;

use crate::{Format, Output};
use forge::core::ForgeError;

pub(crate) fn resolve_module(id: &str) -> Result<PathBuf, ForgeError> {
    forge::module_catalog::resolve(id).map(|module| module.absolute_path)
}

pub(crate) fn cmd_module_list(format: Format) -> Result<Output, ForgeError> {
    let modules = forge::module_catalog::discover()?;
    let json = serde_json::to_value(&modules).expect("module entries serialize");
    let text = if modules.is_empty() {
        "No Forge modules with Gate declarations found.".to_string()
    } else {
        let rows = modules
            .iter()
            .map(|module| {
                format!(
                    "{:<32} {:<12} {:<24} {}",
                    module.id, module.stack, module.profile, module.path
                )
            })
            .collect::<Vec<_>>();
        format!(
            "ID                               STACK        PROFILE                  PATH\n{}",
            rows.join("\n")
        )
    };
    Ok(match format {
        Format::Human | Format::Table => Output::Human(text),
        Format::Json => Output::Json(json),
        Format::Ndjson => Output::Raw(
            modules
                .iter()
                .map(|module| serde_json::to_string(module).expect("module entry serializes"))
                .collect::<Vec<_>>()
                .join("\n")
                + if modules.is_empty() { "" } else { "\n" },
        ),
    })
}
