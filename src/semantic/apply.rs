//! Applying approved metadata outward (`forge classify apply`).
//!
//! The only new write path in the maintainer platform, and it is
//! deliberately narrow:
//!
//! 1. Every selected proposal must already be `Approved`. A
//!    `Suggested` proposal is refused by name — the operator must
//!    approve it first.
//! 2. The approved values are projected onto the four fields
//!    `ALLOWED_PROPOSED_FIELDS` permits. Anything else is refused
//!    rather than silently dropped, because a silently dropped
//!    approval is a lie about what was published.
//! 3. The request is a `forge-metadata-propose/0.1.0` with
//!    `mode: "pr"`. Direct mutation is not reachable from `apply`.
//! 4. The plugin that advertises `kind: metadata` answers with the
//!    reviewed change set and a PR reference.
//! 5. The caller records a journal row naming the plugin, the
//!    fields, and the PR reference. No path, no credential.
//!
//! Forge never mutates a remote directly: the request goes through
//! the configured plugin command over the same bounded-argv,
//! timeout, secret-leak-rejecting transport as publish.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::core::ForgeError;
use crate::github::adapter::ALLOWED_PROPOSED_FIELDS;
use crate::plugins;
use crate::publish::providers::{
    self, MetadataProposeRequest, ProviderEntry, METADATA_PROPOSE_CONTRACT,
};

use super::proposal::ProposalState;
use super::review;

/// What a successful apply produced. The CLI renders this and
/// records the journal row from it.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyOutcome {
    pub plugin_id: String,
    pub fields: Vec<String>,
    pub pr_reference: Option<String>,
    pub changes: Vec<Value>,
    pub note: String,
}

/// Apply the approved proposals for the project at `project_path`
/// through the configured metadata plugin. Refuses unless every
/// selected proposal is `Approved`, every approved value lands on
/// one of the four permitted fields, and a plugin advertising
/// `kind: metadata` is configured and ready.
pub fn apply(project_path: &Path, confirm: bool) -> Result<ApplyOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::SemanticInvalid {
            reason: "apply requires explicit --confirm; nothing was sent to any plugin".to_string(),
        });
    }
    let entries = review::list(project_path)?;
    let mut approved = Vec::new();
    for entry in &entries {
        match entry.state {
            ProposalState::Suggested => {
                return Err(ForgeError::SemanticInvalid {
                    reason: format!(
                        "proposal `{}` (kind `{}`) is still `suggested`; approve it before applying",
                        entry.dir_name,
                        entry.kind.label()
                    ),
                });
            }
            ProposalState::Approved => approved.push(entry),
            _ => {}
        }
    }
    if approved.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "no approved proposals to apply".to_string(),
        });
    }
    let mut fields: BTreeMap<String, Value> = BTreeMap::new();
    for entry in &approved {
        let proposal =
            review::read(project_path, &entry.id)?.ok_or_else(|| ForgeError::SemanticInvalid {
                reason: format!("proposal `{}` was not found", entry.dir_name),
            })?;
        let field = entry.kind.label();
        if !ALLOWED_PROPOSED_FIELDS.contains(&field) {
            return Err(ForgeError::SemanticInvalid {
                reason: format!(
                    "approved proposal `{}` targets field `{field}`, which is outside the permitted set [{}]; refusing rather than dropping it",
                    entry.dir_name,
                    ALLOWED_PROPOSED_FIELDS.join(", ")
                ),
            });
        }
        fields.insert(field.to_string(), Value::String(proposal.suggested_value));
    }
    let field_names: Vec<String> = fields.keys().cloned().collect();
    let project_id = approved[0].project_id.clone();

    let config_path = plugins::resolve_config_path(Some(project_path));
    let provider_config = if config_path.is_file() {
        providers::load_config(&config_path)?
    } else {
        providers::ProviderConfig::default()
    };
    let descriptors = if config_path.is_file() {
        plugins::load_descriptors(&config_path)?
    } else {
        plugins::PluginConfig::default()
    };
    let records = plugins::list(&provider_config, &descriptors);
    let candidates = plugins::metadata_plugins(&records, &field_names);
    if candidates.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: format!(
                "no configured plugin advertises `kind: metadata` for the approved fields [{}]; cannot apply",
                field_names.join(", ")
            ),
        });
    }
    let plugin = candidates[0];
    let request = MetadataProposeRequest {
        contract: METADATA_PROPOSE_CONTRACT.to_string(),
        operation_id: format!("classify-apply-{}", chrono::Utc::now().timestamp()),
        project_id,
        fields,
        mode: "pr".to_string(),
    };
    let entry = ProviderEntry {
        id: plugin.id.clone(),
        command: plugin.command.clone(),
        enabled: plugin.enabled,
    };
    let response = providers::invoke_metadata_provider(&entry, &request, project_path)?;
    Ok(ApplyOutcome {
        plugin_id: plugin.id.clone(),
        fields: field_names,
        pr_reference: response.pr,
        changes: response.changes,
        note: response.note.unwrap_or_default(),
    })
}
