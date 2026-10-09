//! The plugin registry: a named view over the configured external
//! integrations, so GitHub, OpenPanel and any future remote are
//! enumerated the same way instead of being hard-coded special cases.
//!
//! # Boundary
//!
//! This module owns *naming and capability discovery*. It does not
//! own invocation: every plugin is still run by
//! [`crate::publish::providers::invoke_provider`], over the same
//! JSON-on-stdin/stdout transport, with the same bounded argv,
//! per-run timeout and secret-leak rejection. A plugin gains no new
//! privilege by being listed here.
//!
//! # Why a descriptor
//!
//! A configured plugin is `{id, command, enabled}` — that is the whole
//! shape every existing `providers.yaml` in the wild has, and the one
//! `delivery` already resolves `openpanel` from. Requiring a
//! descriptor would break every existing configuration, so a plugin
//! with no descriptor is treated as a [`PluginKind::Delivery`]
//! plugin: the default is what already ships.
//!
//! Where a descriptor *is* present it adds three optional fields —
//! `kind`, `capabilities`, `description` — so the registry can answer
//! "which plugin maintains metadata" without hard-coding an id.
//!
//! # Capability vocabulary
//!
//! Capabilities are a closed set. A descriptor naming anything else is
//! reported [`PluginState::Invalid`] rather than silently accepted: an
//! unknown capability in a registry that claims to enumerate the
//! workspace's reach is worse than no registry at all, because the
//! operator cannot tell which claims are trustworthy.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::publish::providers::{ProviderConfig, ProviderEntry};

/// The closed capability vocabulary. A plugin may advertise any
/// subset; anything outside this set is invalid.
pub const CAPABILITIES: [&str; 11] = [
    "topic",
    "description",
    "homepage",
    "language",
    "publish",
    "delivery",
    "gate",
    "quality",
    "agent",
    "contract",
    "analytics",
];

/// What a plugin is *for*, independent of which capabilities it
/// advertises. `delivery` is the default for an existing
/// configuration that carries no descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    /// Reads and proposes changes to a hosted repository's metadata.
    Metadata,
    /// Builds, publishes or promotes a deployed artifact.
    Delivery,
    /// Runs policy/gate checks (driftwatchdog family).
    Gate,
    /// Runs quality checks (cargo-* family).
    Quality,
    /// Runs agent sessions (sisyphusfy/ariadex/mnemora family).
    Agent,
    /// Serves platform contracts (platform-contracts family).
    Contract,
}

impl PluginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginKind::Metadata => "metadata",
            PluginKind::Delivery => "delivery",
            PluginKind::Gate => "gate",
            PluginKind::Quality => "quality",
            PluginKind::Agent => "agent",
            PluginKind::Contract => "contract",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "metadata" => Some(PluginKind::Metadata),
            "delivery" => Some(PluginKind::Delivery),
            "gate" => Some(PluginKind::Gate),
            "quality" => Some(PluginKind::Quality),
            "agent" => Some(PluginKind::Agent),
            "contract" => Some(PluginKind::Contract),
            _ => None,
        }
    }
}

/// Optional per-plugin declaration. Every field is optional, so a
/// descriptor carrying only `kind` is still valid, and its absence
/// entirely is also valid.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PluginDescriptor {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// The configuration shape: the existing `providers:` list, plus an
/// optional parallel `plugins:` list carrying descriptors.
///
/// Descriptors may be declared either as a sibling `plugins:` block
/// (one entry per id) or, for convenience, inline on the provider via
/// a `descriptor:` key. The sibling block is the documented form; the
/// inline key exists so a single-plugin config file stays a single
/// file.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PluginConfig {
    #[serde(default)]
    pub plugins: Vec<PluginDescriptorEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PluginDescriptorEntry {
    pub id: String,
    #[serde(default, flatten)]
    pub descriptor: PluginDescriptor,
}

/// Why a plugin is not usable right now. Every variant names the
/// plugin: a registry that reports "something is wrong" without
/// saying which thing is the opposite of useful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    /// Configured, executable, capabilities verified.
    Ready,
    /// Declared in the config but switched off by the operator.
    Disabled,
    /// The declared command is not executable on this host.
    Unavailable { reason: String },
    /// The descriptor is malformed — an unknown kind or a capability
    /// outside the closed vocabulary.
    Invalid { reason: String },
}

impl PluginState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginState::Ready => "ready",
            PluginState::Disabled => "disabled",
            PluginState::Unavailable { .. } => "unavailable",
            PluginState::Invalid { .. } => "invalid",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            PluginState::Unavailable { reason } | PluginState::Invalid { reason } => Some(reason),
            _ => None,
        }
    }
}

/// One row of `forge plugins`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRecord {
    pub id: String,
    pub kind: PluginKind,
    pub enabled: bool,
    pub command: PathBuf,
    /// Only the capabilities that passed the closed-vocabulary check.
    pub capabilities: BTreeSet<String>,
    pub state: PluginState,
    pub description: Option<String>,
}

impl PluginRecord {
    /// Capabilities this plugin actually advertises, sorted, for a
    /// stable render.
    pub fn capability_list(&self) -> Vec<String> {
        self.capabilities.iter().cloned().collect()
    }

    /// Whether this plugin can service a metadata request for the
    /// given field. Used to route `classify apply`.
    pub fn maintains(&self, field: &str) -> bool {
        self.kind == PluginKind::Metadata && self.capabilities.contains(field)
    }
}

/// The default config path, matching the one `delivery` already uses,
/// so one file describes every plugin rather than two files drifting.
pub fn resolve_config_path(project_dir: Option<&Path>) -> PathBuf {
    if let Some(value) = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG") {
        let trimmed = value.to_string_lossy().trim().to_string();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    match project_dir {
        Some(dir) => dir.join(".forge/providers.yaml"),
        None => PathBuf::from(".forge/providers.yaml"),
    }
}

/// Read the optional sibling `plugins:` block from a provider
/// config file. A file with no `plugins:` key yields an empty
/// descriptor set — every plugin is then a delivery plugin, which
/// is what every configuration that exists today already means.
pub fn load_descriptors(path: &Path) -> Result<PluginConfig, crate::core::ForgeError> {
    let bytes = std::fs::read(path).map_err(|error| crate::core::ForgeError::PublishInvalid {
        reason: format!(
            "cannot read publish provider config {}: {error}",
            path.display()
        ),
    })?;
    serde_yaml::from_slice::<PluginConfig>(&bytes).map_err(|error| {
        crate::core::ForgeError::PublishInvalid {
            reason: format!(
                "invalid plugin descriptor block in {}: {error}",
                path.display()
            ),
        }
    })
}

/// Enumerate every configured plugin with its verified state.
///
/// A plugin whose command is missing is reported `Unavailable` and the
/// remaining plugins are still listed: one broken entry must not hide
/// the rest of the workspace's reach.
///
/// Resolution order per id: `providers.yaml` `plugins:` descriptor first
/// (explicit override), then the builtin table consulting
/// `kits/manifest.json` when it carries a `plugins` array, then the
/// hardcoded builtin map, then the legacy delivery default.
pub fn list(config: &ProviderConfig, descriptors: &PluginConfig) -> Vec<PluginRecord> {
    let mut records = Vec::with_capacity(config.providers.len());
    for entry in &config.providers {
        let explicit = descriptor_for(descriptors, &entry.id);
        if explicit.is_some() {
            records.push(record_for(entry, explicit));
        } else if let Some(builtin) = builtin_descriptor(&entry.id) {
            records.push(record_for(entry, Some(&builtin)));
        } else {
            records.push(record_for(entry, None));
        }
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    records
}

/// The plugins that can service a metadata propose request for every
/// field named, in registry order.
pub fn metadata_plugins<'a>(
    records: &'a [PluginRecord],
    fields: &[String],
) -> Vec<&'a PluginRecord> {
    records
        .iter()
        .filter(|record| {
            record.state == PluginState::Ready
                && record.enabled
                && fields.iter().all(|field| record.maintains(field))
        })
        .collect()
}

/// Ready, enabled gate plugins in registry order, beside `metadata_plugins`.
pub fn gate_plugins<'a>(records: &'a [PluginRecord]) -> Vec<&'a PluginRecord> {
    plugins_of_kind(records, PluginKind::Gate)
}

/// Ready, enabled quality plugins in registry order.
pub fn quality_plugins<'a>(records: &'a [PluginRecord]) -> Vec<&'a PluginRecord> {
    plugins_of_kind(records, PluginKind::Quality)
}

/// Ready, enabled agent plugins in registry order.
pub fn agent_plugins<'a>(records: &'a [PluginRecord]) -> Vec<&'a PluginRecord> {
    plugins_of_kind(records, PluginKind::Agent)
}

/// Ready, enabled contract plugins in registry order.
pub fn contract_plugins<'a>(records: &'a [PluginRecord]) -> Vec<&'a PluginRecord> {
    plugins_of_kind(records, PluginKind::Contract)
}

fn plugins_of_kind<'a>(records: &'a [PluginRecord], kind: PluginKind) -> Vec<&'a PluginRecord> {
    records
        .iter()
        .filter(|record| {
            record.state == PluginState::Ready && record.enabled && record.kind == kind
        })
        .collect()
}

/// Builtin kind/capability table for well-known plugin ids.
///
/// `kits/manifest.json` is consulted first: when it exists and carries a
/// `plugins: [{id, kind, capabilities, description}]` array, a matching
/// entry wins over the hardcoded map below. A `providers.yaml`
/// descriptor always overrides both (see `list`). An absent file, an
/// unreadable file, or a file without a `plugins` key falls back to the
/// hardcoded map, which is the documented builtin set:
///
/// driftwatchdog=gate, cargo-*=quality, sisyphusfy/ariadex/mnemora=agent,
/// platform-contracts=contract, labrys/openpanel/jenkins-local/jenkins=delivery,
/// argoscope/devloom=metadata+analytics.
pub fn builtin_descriptor(id: &str) -> Option<PluginDescriptor> {
    if let Some(from_manifest) = builtin_from_kits_manifest(id) {
        return Some(from_manifest);
    }
    builtin_hardcoded(id)
}

fn builtin_hardcoded(id: &str) -> Option<PluginDescriptor> {
    let (kind, capabilities): (&str, &[&str]) = if id == "driftwatchdog" {
        ("gate", &["gate"])
    } else if id.starts_with("cargo-") {
        ("quality", &["quality"])
    } else if id == "sisyphusfy" || id == "ariadex" || id == "mnemora" {
        ("agent", &["agent"])
    } else if id == "platform-contracts" {
        ("contract", &["contract"])
    } else if id == "labrys" || id == "openpanel" || id == "jenkins-local" || id == "jenkins" {
        ("delivery", &["delivery"])
    } else if id == "argoscope" || id == "devloom" {
        ("metadata", &["analytics"])
    } else {
        return None;
    };
    Some(PluginDescriptor {
        kind: Some(kind.to_string()),
        capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
        description: Some(format!("builtin {kind} plugin `{id}`")),
    })
}

fn kits_manifest_path() -> PathBuf {
    if let Some(value) = std::env::var_os("FORGE_KITS_MANIFEST") {
        let trimmed = value.to_string_lossy().trim().to_string();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from("kits/manifest.json")
}

fn builtin_from_kits_manifest(id: &str) -> Option<PluginDescriptor> {
    let path = kits_manifest_path();
    let bytes = std::fs::read(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let plugins = value.get("plugins")?.as_array()?;
    for entry in plugins {
        if entry.get("id")?.as_str()? != id {
            continue;
        }
        let kind = entry
            .get("kind")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let capabilities = entry
            .get("capabilities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let description = entry
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        // Only honor closed-vocabulary entries; anything else falls
        // through to the hardcoded table so an edited feed cannot
        // smuggle an unknown capability into Ready.
        if let Some(ref k) = kind {
            if PluginKind::parse(k).is_none() {
                continue;
            }
        }
        if capabilities
            .iter()
            .any(|c| !CAPABILITIES.contains(&c.as_str()))
        {
            continue;
        }
        return Some(PluginDescriptor {
            kind,
            capabilities,
            description,
        });
    }
    None
}

fn descriptor_for<'a>(descriptors: &'a PluginConfig, id: &str) -> Option<&'a PluginDescriptor> {
    descriptors
        .plugins
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| &entry.descriptor)
}

fn record_for(entry: &ProviderEntry, descriptor: Option<&PluginDescriptor>) -> PluginRecord {
    // An absent descriptor is a delivery plugin: the shape every
    // configuration that exists today already has.
    let (kind, capabilities, description, descriptor_problem) = match descriptor {
        None => (
            PluginKind::Delivery,
            BTreeSet::from(["delivery".to_string()]),
            None,
            None,
        ),
        Some(descriptor) => {
            let kind = match descriptor.kind.as_deref() {
                None => PluginKind::Delivery,
                Some(raw) => match PluginKind::parse(raw) {
                    Some(kind) => kind,
                    None => {
                        return invalid_record(
                            entry,
                            PluginKind::Delivery,
                            format!(
                                "unknown plugin kind `{raw}`; expected one of metadata|delivery|gate|quality|agent|contract"
                            ),
                        )
                    }
                },
            };
            let mut capabilities = BTreeSet::new();
            let mut unknown = Vec::new();
            for capability in &descriptor.capabilities {
                if CAPABILITIES.contains(&capability.as_str()) {
                    capabilities.insert(capability.clone());
                } else {
                    unknown.push(capability.clone());
                }
            }
            if !unknown.is_empty() {
                return invalid_record(
                    entry,
                    kind,
                    format!(
                        "plugin `{}` declares capability {} outside the closed set [{}]",
                        entry.id,
                        unknown
                            .iter()
                            .map(|c| format!("`{c}`"))
                            .collect::<Vec<_>>()
                            .join(", "),
                        CAPABILITIES.join(", ")
                    ),
                );
            }
            if capabilities.is_empty() {
                capabilities.insert(kind.as_str().to_string());
            }
            let problem = None::<String>;
            (kind, capabilities, descriptor.description.clone(), problem)
        }
    };

    let state = if !entry.enabled {
        PluginState::Disabled
    } else if !executable(&entry.command) {
        PluginState::Unavailable {
            reason: format!(
                "command `{}` is not an executable file",
                entry.command.display()
            ),
        }
    } else {
        match descriptor_problem {
            Some(reason) => PluginState::Invalid { reason },
            None => PluginState::Ready,
        }
    };

    PluginRecord {
        id: entry.id.clone(),
        kind,
        enabled: entry.enabled,
        command: entry.command.clone(),
        capabilities,
        state,
        description,
    }
}

fn invalid_record(entry: &ProviderEntry, kind: PluginKind, reason: String) -> PluginRecord {
    PluginRecord {
        id: entry.id.clone(),
        kind,
        enabled: entry.enabled,
        command: entry.command.clone(),
        // An invalid descriptor advertises nothing: the point is that
        // the operator cannot trust these claims.
        capabilities: BTreeSet::new(),
        state: PluginState::Invalid { reason },
        description: None,
    }
}

fn executable(command: &Path) -> bool {
    if !command.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return std::fs::metadata(command)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str, command: &str, enabled: bool) -> ProviderEntry {
        ProviderEntry {
            id: id.to_string(),
            command: PathBuf::from(command),
            enabled,
        }
    }

    fn descriptor(id: &str, kind: &str, capabilities: &[&str]) -> PluginConfig {
        PluginConfig {
            plugins: vec![PluginDescriptorEntry {
                id: id.to_string(),
                descriptor: PluginDescriptor {
                    kind: Some(kind.to_string()),
                    capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
                    description: None,
                },
            }],
        }
    }

    #[test]
    fn a_plugin_without_a_descriptor_is_a_delivery_plugin() {
        // This is the shape every existing providers.yaml has. It must keep
        // working, so it resolves to delivery without any edit.
        let config = ProviderConfig {
            providers: vec![provider("openpanel", "/bin/true", true)],
        };
        let records = list(&config, &PluginConfig::default());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, PluginKind::Delivery);
        assert_eq!(records[0].state, PluginState::Ready);
        assert_eq!(records[0].capability_list(), vec!["delivery"]);
    }

    #[test]
    fn a_metadata_plugin_advertises_its_fields() {
        let config = ProviderConfig {
            providers: vec![provider("github", "/bin/true", true)],
        };
        let records = list(
            &config,
            &descriptor("github", "metadata", &["topic", "description"]),
        );
        assert_eq!(records[0].kind, PluginKind::Metadata);
        assert!(records[0].maintains("topic"));
        assert!(records[0].maintains("description"));
        assert!(!records[0].maintains("homepage"));
    }

    #[test]
    fn an_unknown_capability_is_invalid_and_advertises_nothing() {
        let config = ProviderConfig {
            providers: vec![provider("github", "/bin/true", true)],
        };
        let records = list(
            &config,
            &descriptor("github", "metadata", &["topic", "teleport"]),
        );
        assert!(matches!(records[0].state, PluginState::Invalid { .. }));
        assert!(records[0].capabilities.is_empty());
        let reason = records[0].state.reason().unwrap();
        assert!(reason.contains("teleport"), "{reason}");
        assert!(reason.contains("closed set"), "{reason}");
    }

    #[test]
    fn an_unknown_kind_is_invalid() {
        let config = ProviderConfig {
            providers: vec![provider("x", "/bin/true", true)],
        };
        let records = list(&config, &descriptor("x", "teleport", &[]));
        assert!(matches!(records[0].state, PluginState::Invalid { .. }));
        assert!(records[0]
            .state
            .reason()
            .unwrap()
            .contains("unknown plugin kind"));
    }

    #[test]
    fn a_missing_command_does_not_hide_the_others() {
        let config = ProviderConfig {
            providers: vec![
                provider("broken", "/nonexistent/plugin", true),
                provider("working", "/bin/true", true),
            ],
        };
        let records = list(&config, &PluginConfig::default());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "broken");
        assert!(matches!(records[0].state, PluginState::Unavailable { .. }));
        // The healthy plugin is still listed.
        assert_eq!(records[1].id, "working");
        assert_eq!(records[1].state, PluginState::Ready);
    }

    #[test]
    fn a_disabled_plugin_is_reported_disabled_not_broken() {
        let config = ProviderConfig {
            providers: vec![provider("off", "/bin/true", false)],
        };
        let records = list(&config, &PluginConfig::default());
        assert_eq!(records[0].state, PluginState::Disabled);
    }

    #[test]
    fn metadata_routing_requires_every_field() {
        let config = ProviderConfig {
            providers: vec![provider("gh", "/bin/true", true)],
        };
        let records = list(
            &config,
            &descriptor("gh", "metadata", &["topic", "description"]),
        );
        let both = metadata_plugins(&records, &["topic".to_string(), "description".to_string()]);
        assert_eq!(both.len(), 1);
        let too_many = metadata_plugins(&records, &["topic".to_string(), "homepage".to_string()]);
        assert!(too_many.is_empty(), "no plugin can service `homepage`");
    }
}
