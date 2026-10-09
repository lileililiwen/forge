//! Plugin commands (`plugins`).
//!
//! Typed CLI handlers for plugin listing over provider configs.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::plugins;
use forge::publish::providers;
use std::path::Path;

use super::commands::PluginsCommands;
use super::constants::PLUGINS_CONTRACT;
use crate::{Format, Output};

/// Read the optional sibling `plugins:` block from a provider config.
///
/// The block is parsed from the same file as `providers:` rather than a
/// second file, so one config file describes every plugin and the two lists
/// cannot drift apart. A file with no `plugins:` key yields an empty
/// descriptor set — every plugin is then a delivery plugin, which is what
/// every configuration that exists today already means.
fn load_plugin_descriptors(path: &Path) -> Result<plugins::PluginConfig, ForgeError> {
    plugins::load_descriptors(path)
}

pub(crate) fn cmd_plugins(
    db_path: &Path,
    command: &PluginsCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PluginsCommands::List { project } => {
            // A missing config is an honest empty registry, not a failure:
            // "no plugins configured" is a real answer, and an operator with
            // no providers.yaml yet should see that, not a stack trace.
            let dir = Path::new(project);
            let config_path = plugins::resolve_config_path(Some(dir));
            if !config_path.is_file() {
                // The JSON contract holds on this path too: "no plugins
                // configured" is an answer, not a failure, and a caller
                // asking for JSON must get JSON.
                if matches!(format, Format::Json | Format::Ndjson) {
                    return Ok(Output::Json(serde_json::json!({
                        "contract": PLUGINS_CONTRACT,
                        "config": config_path.to_string_lossy(),
                        "plugins": [],
                    })));
                }
                let mut lines = vec!["No plugins configured.".to_string()];
                lines.push(format!("Looked for {}", config_path.display()));
                lines.push(
                    "Add a `providers:` entry to declare one; a plugin without a \
                     descriptor is treated as a delivery plugin."
                        .to_string(),
                );
                return Ok(Output::Human(lines.join("\n")));
            }
            let provider_config = providers::load_config(&config_path)?;
            // Descriptors live in a sibling `plugins:` block in the same file.
            // A file without one is the existing shape: every plugin is then a
            // delivery plugin, which is what every configuration in the wild
            // already means.
            let descriptors = load_plugin_descriptors(&config_path)?;
            let records = plugins::list(&provider_config, &descriptors);
            let _ = db_path;
            match format {
                Format::Json | Format::Ndjson => {
                    let payload = serde_json::json!({
                        "contract": PLUGINS_CONTRACT,
                        "config": config_path.to_string_lossy(),
                        "plugins": records
                            .iter()
                            .map(|record| serde_json::json!({
                                "id": record.id,
                                "kind": record.kind.as_str(),
                                "enabled": record.enabled,
                                "state": record.state.as_str(),
                                "reason": record.state.reason(),
                                "command": record.command.to_string_lossy(),
                                "capabilities": record.capability_list(),
                                "description": record.description,
                            }))
                            .collect::<Vec<_>>(),
                    });
                    Ok(Output::Json(payload))
                }
                _ => {
                    if records.is_empty() {
                        return Ok(Output::Human(format!(
                            "No plugins configured in {}",
                            config_path.display()
                        )));
                    }
                    let mut lines = Vec::new();
                    for record in &records {
                        lines.push(format!(
                            "{}  {}  {}{}",
                            record.id,
                            record.kind.as_str(),
                            record.state.as_str(),
                            match record.state.reason() {
                                Some(reason) => format!("  ({reason})"),
                                None => String::new(),
                            }
                        ));
                        lines.push(format!("    command: {}", record.command.to_string_lossy()));
                        lines.push(format!(
                            "    capabilities: {}",
                            if record.capabilities.is_empty() {
                                "none (this plugin advertises nothing it can be trusted with)"
                                    .to_string()
                            } else {
                                record.capability_list().join(", ")
                            }
                        ));
                        if let Some(description) = &record.description {
                            lines.push(format!("    {description}"));
                        }
                    }
                    Ok(Output::Human(lines.join("\n")))
                }
            }
        }
    }
}
