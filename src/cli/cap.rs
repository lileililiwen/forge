//! Capability grouping facade (`forge cap`).
//!
//! Thin grouping over the existing flat commands: gate/check/doctor/readiness
//! under gate, agent/studio/intent under agent, contract/component/standard
//! under contract, delivery/deploy/publish under delivery (plus quality and
//! analytics rows). No flat command is removed or re-dispatched; `cap run`
//! prints the exact underlying `forge …` string so the operator reviews
//! before running. `cap list` derives from `CAPABILITIES` plus live
//! `plugins list` states.

use std::path::Path;

use forge::core::ForgeError;
use forge::plugins;
use forge::publish::providers;

use super::commands::CapCommands;
use crate::{Format, Output};

pub(super) const CAP_CONTRACT: &str = "forge-cap/0.1.0";

/// Flat commands grouped under each capability, per the brief.
pub fn cap_group_commands(capability: &str) -> Option<&'static [&'static str]> {
    match capability {
        "gate" => Some(&["gate", "check", "doctor", "readiness"]),
        "agent" => Some(&["agent", "studio", "intent"]),
        "contract" => Some(&["contract", "component", "standard"]),
        "delivery" => Some(&["delivery", "deploy", "publish"]),
        "quality" => Some(&["check", "doctor"]),
        "analytics" => Some(&["analytics"]),
        "metadata" => Some(&["classify", "describe"]),
        _ => None,
    }
}

fn live_records(project: &str) -> Vec<plugins::PluginRecord> {
    let dir = Path::new(project);
    let config_path = plugins::resolve_config_path(Some(dir));
    let Ok(provider_config) = providers::load_config(&config_path) else {
        return Vec::new();
    };
    let descriptors = plugins::load_descriptors(&config_path).unwrap_or_default();
    plugins::list(&provider_config, &descriptors)
}

pub(crate) fn cmd_cap(
    _db_path: &Path,
    command: &CapCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        CapCommands::List { project } => {
            let records = live_records(project);
            let ready = |cap: &str| {
                records
                    .iter()
                    .filter(|r| {
                        r.state == plugins::PluginState::Ready
                            && r.enabled
                            && r.capabilities.contains(cap)
                    })
                    .count()
            };
            match format {
                Format::Json | Format::Ndjson => {
                    let payload = serde_json::json!({
                        "contract": CAP_CONTRACT,
                        "groups": plugins::CAPABILITIES.iter().map(|cap| {
                            serde_json::json!({
                                "capability": cap,
                                "ready_plugins": ready(cap),
                                "commands": cap_group_commands(cap).unwrap_or(&[]),
                            })
                        }).collect::<Vec<_>>(),
                    });
                    Ok(Output::Json(payload))
                }
                _ => {
                    let mut lines = vec![
                        "Capability groups (live Ready counts from `forge plugins list`):"
                            .to_string(),
                    ];
                    for cap in plugins::CAPABILITIES {
                        let cmds = cap_group_commands(cap)
                            .map(|c| c.join(", "))
                            .unwrap_or_default();
                        lines.push(format!("{cap}  ready={}  commands: {cmds}", ready(cap)));
                    }
                    lines.push(
                        "Old flat commands still work: gate, check, doctor, readiness, agent, studio, intent, contract, component, standard, delivery, deploy, publish.".to_string(),
                    );
                    Ok(Output::Human(lines.join("\n")))
                }
            }
        }
        CapCommands::Inspect {
            capability,
            project,
        } => {
            let cap = capability.trim().to_lowercase();
            let Some(grouped) = cap_group_commands(&cap) else {
                return Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "cap-invalid: unknown capability `{capability}`; expected one of {}",
                        plugins::CAPABILITIES.join(", ")
                    ),
                });
            };
            let records = live_records(project);
            let providers: Vec<String> = records
                .iter()
                .filter(|r| {
                    r.state == plugins::PluginState::Ready
                        && r.enabled
                        && r.capabilities.contains(cap.as_str())
                })
                .map(|r| format!("{} ({})", r.id, r.kind.as_str()))
                .collect();
            match format {
                Format::Json | Format::Ndjson => Ok(Output::Json(serde_json::json!({
                    "contract": CAP_CONTRACT,
                    "capability": cap,
                    "commands": grouped,
                    "ready_plugins": providers,
                }))),
                _ => {
                    let plugins_line = if providers.is_empty() {
                        "no Ready plugins advertise this capability".to_string()
                    } else {
                        providers.join(", ")
                    };
                    Ok(Output::Human(format!(
                        "capability: {cap}\ncommands: {}\nready plugins: {plugins_line}",
                        grouped.join(", ")
                    )))
                }
            }
        }
        CapCommands::Add { capability, id } => {
            let cap = capability.trim().to_lowercase();
            if !plugins::CAPABILITIES.contains(&cap.as_str()) {
                return Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "cap-invalid: unknown capability `{capability}`; expected one of {}",
                        plugins::CAPABILITIES.join(", ")
                    ),
                });
            }
            let snippet = format!(
                "Add plugin `{id}` to group `{cap}` in `.forge/providers.yaml`:\nproviders:\n  - id: {id}\n    command: ./adapters/{id}\n    enabled: true\nplugins:\n  - id: {id}\n    kind: {cap}\n    capabilities: [{cap}]\nOld flat commands still work; `cap add` writes nothing."
            );
            match format {
                Format::Json | Format::Ndjson => Ok(Output::Json(serde_json::json!({
                    "contract": CAP_CONTRACT,
                    "capability": cap,
                    "id": id,
                    "snippet": snippet,
                }))),
                _ => Ok(Output::Human(snippet)),
            }
        }
        CapCommands::Run { capability, action } => {
            let cap = capability.trim().to_lowercase();
            let Some(grouped) = cap_group_commands(&cap) else {
                return Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "cap-invalid: unknown capability `{capability}`; expected one of {}",
                        plugins::CAPABILITIES.join(", ")
                    ),
                });
            };
            let primary = grouped.first().unwrap_or(&"gate");
            let hint = format!(
                "forge {primary} {action}  (group `{cap}` maps to: {}; run the flat command directly — `cap run` never executes it)"
                    ,
                grouped.join(", ")
            );
            match format {
                Format::Json | Format::Ndjson => Ok(Output::Json(serde_json::json!({
                    "contract": CAP_CONTRACT,
                    "capability": cap,
                    "command": format!("forge {primary} {action}"),
                    "group": grouped,
                }))),
                _ => Ok(Output::Human(hint)),
            }
        }
    }
}
