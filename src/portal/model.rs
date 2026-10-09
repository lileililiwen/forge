//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::fleet::FleetReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use super::constants::{
    SECTION_AGENTS, SECTION_ANALYTICS, SECTION_COMPONENTS, SECTION_DEPLOYMENTS,
    SECTION_DOCUMENTATION, SECTION_FEATURES, SECTION_POLICIES, SECTION_PROJECTS,
    SECTION_REPOSITORIES, SECTION_SERVERS, SECTION_SETTINGS, SECTION_SPECS,
};

/// Bounded portal section. The enum is the wire-level
/// contract; the JSON envelope always carries the kebab
/// case id and the human renderer always carries the
/// title-cased label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortalSection {
    Projects,
    Features,
    Components,
    Policies,
    Specs,
    Agents,
    Deployments,
    Repositories,
    Documentation,
    Analytics,
    Servers,
    Settings,
}
impl PortalSection {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            PortalSection::Projects => SECTION_PROJECTS,
            PortalSection::Features => SECTION_FEATURES,
            PortalSection::Components => SECTION_COMPONENTS,
            PortalSection::Policies => SECTION_POLICIES,
            PortalSection::Specs => SECTION_SPECS,
            PortalSection::Agents => SECTION_AGENTS,
            PortalSection::Deployments => SECTION_DEPLOYMENTS,
            PortalSection::Repositories => SECTION_REPOSITORIES,
            PortalSection::Documentation => SECTION_DOCUMENTATION,
            PortalSection::Analytics => SECTION_ANALYTICS,
            PortalSection::Servers => SECTION_SERVERS,
            PortalSection::Settings => SECTION_SETTINGS,
        }
    }
    /// Title-cased label for the human renderer. The
    /// string is the section heading; entries render below
    /// it.
    pub fn label(self) -> &'static str {
        match self {
            PortalSection::Projects => "Projects",
            PortalSection::Features => "Features",
            PortalSection::Components => "Components",
            PortalSection::Policies => "Policies",
            PortalSection::Specs => "Specs",
            PortalSection::Agents => "Agents",
            PortalSection::Deployments => "Deployments",
            PortalSection::Repositories => "Repositories",
            PortalSection::Documentation => "Documentation",
            PortalSection::Analytics => "Analytics",
            PortalSection::Servers => "Servers",
            PortalSection::Settings => "Settings",
        }
    }
}
/// How the (optionally configured) workspace fleet registry projects
/// into the portal. `None` (no registry configured) renders no fleet
/// block at all, so every local portal workflow stays byte-identical;
/// a configured registry always renders, and its source status is
/// never masked as `ok` when it is stale or could not be observed.
#[derive(Debug, Clone)]
pub enum FleetProjection<'a> {
    /// A configured registry observed successfully.
    Configured(&'a FleetReport),
    /// A configured registry that could not be observed (malformed or
    /// unreadable document). The block renders `unavailable` with the
    /// typed code and reason named.
    Failed { code: &'static str, reason: String },
}
/// One recent operation surfaced in the dashboard. The
/// portal reads the registry's `operations` table; the
/// entry is never fabricated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalOperation {
    pub op_id: i64,
    pub kind: String,
    pub project_id: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
/// Operator-facing portal configuration. The `manifest`
/// block is the source of truth; the validator is the
/// only path that produces this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalConfig {
    pub enabled: bool,
    /// Optional title for the dashboard. Defaults to
    /// `"Forge Control Plane"` when the manifest omits
    /// the field.
    pub title: String,
    /// Default scope: per-project or fleet-wide.
    pub default_scope: PortalScope,
}
impl PortalConfig {
    /// Build a configuration from the parsed manifest. A
    /// missing `portal:` block returns the default
    /// configuration; a malformed block is refused with
    /// the typed `portal-invalid` code.
    pub fn from_manifest(manifest: &Manifest) -> Result<Self, ForgeError> {
        let raw = match manifest.portal.as_ref() {
            Some(meta) => meta,
            None => return Ok(PortalConfig::default()),
        };
        if !raw.enabled && raw.title.is_some() {
            return Err(ForgeError::PortalInvalid {
                reason: "portal block sets `title` while `enabled: false`; remove \
                         `title` or set `enabled: true` to render the dashboard"
                    .to_string(),
            });
        }
        let default_scope = match raw.default_scope.as_deref() {
            None => PortalScope::Project,
            Some("project") => PortalScope::Project,
            Some("fleet") => PortalScope::Fleet,
            Some(other) => {
                return Err(ForgeError::PortalInvalid {
                    reason: format!(
                        "unknown portal `default_scope` `{other}`; supported: project, fleet"
                    ),
                });
            }
        };
        if let Some(title) = raw.title.as_deref() {
            if title.trim().is_empty() {
                return Err(ForgeError::PortalInvalid {
                    reason: "portal `title` is empty".to_string(),
                });
            }
            if title.len() > 128 {
                return Err(ForgeError::PortalInvalid {
                    reason: format!("portal `title` is {len} chars; max 128", len = title.len()),
                });
            }
        }
        Ok(PortalConfig {
            enabled: raw.enabled,
            title: raw
                .title
                .clone()
                .unwrap_or_else(|| "Forge Control Plane".to_string()),
            default_scope,
        })
    }
    /// Load the configuration from a project directory.
    /// Returns `None` when the directory is not a Forge
    /// project (no manifest, no `portal:` block is treated
    /// as the default and surfaces as `Some(default)`).
    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>, ForgeError> {
        let (manifest, _) = match Manifest::load_from_dir(dir, None) {
            Ok(value) => value,
            Err(ForgeError::ManifestNotFound { .. }) => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(Some(Self::from_manifest(&manifest)?))
    }
}
/// Portal scope. The contract is intentionally small: the
/// portal either renders one project or the entire
/// registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortalScope {
    #[default]
    Project,
    Fleet,
}
impl PortalScope {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            PortalScope::Project => "project",
            PortalScope::Fleet => "fleet",
        }
    }
}
/// One entry on a portal view: a stable id, a human label,
/// a status (ok / warn / fail / unknown / unavailable /
/// partial), a timestamped source, the underlying module
/// the entry was read from and an optional list of
/// evidence lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalEntry {
    pub id: String,
    pub label: String,
    pub status: PortalStatus,
    pub source: String,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
}
/// Per-section view. The renderer consumes one struct per
/// `forge portal view <section>` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalSectionView {
    pub contract: String,
    pub generated_at: String,
    pub project_id: Option<String>,
    pub scope: PortalScope,
    pub section: PortalSection,
    pub section_id: String,
    pub status: PortalStatus,
    pub source: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<PortalEntry>,
    pub controls_available: Vec<String>,
}
impl PortalSectionView {
    pub(super) fn rollup_status(entries: &[PortalEntry]) -> PortalStatus {
        let mut worst = PortalStatus::Ok;
        for entry in entries {
            let rank = match entry.status {
                PortalStatus::Ok => 0,
                PortalStatus::Partial => 1,
                PortalStatus::Warn => 2,
                PortalStatus::Unknown => 3,
                PortalStatus::Unavailable => 4,
                PortalStatus::Fail => 5,
            };
            if rank
                > match worst {
                    PortalStatus::Ok => 0,
                    PortalStatus::Partial => 1,
                    PortalStatus::Warn => 2,
                    PortalStatus::Unknown => 3,
                    PortalStatus::Unavailable => 4,
                    PortalStatus::Fail => 5,
                }
            {
                worst = entry.status;
            }
        }
        worst
    }
}
/// Stable portal status. The contract refuses to claim
/// "healthy" when the underlying module reports unknown,
/// unavailable or partial. A dashboard that implies
/// stronger health than the evidence supports is the
/// change-specific risk the spec calls out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortalStatus {
    Ok,
    Warn,
    Fail,
    Unknown,
    Unavailable,
    Partial,
}
impl PortalStatus {
    /// Wire-level kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            PortalStatus::Ok => "ok",
            PortalStatus::Warn => "warn",
            PortalStatus::Fail => "fail",
            PortalStatus::Unknown => "unknown",
            PortalStatus::Unavailable => "unavailable",
            PortalStatus::Partial => "partial",
        }
    }
}
/// Fleet-wide dashboard. The renderer consumes one struct
/// per `forge portal dashboard` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalDashboard {
    pub contract: String,
    pub generated_at: String,
    pub title: String,
    pub scope: PortalScope,
    pub project_id: Option<String>,
    pub section_count: usize,
    pub sections: Vec<PortalSectionView>,
    pub operations: Vec<PortalOperation>,
}
impl PortalDashboard {
    /// Roll-up status across every section. The dashboard
    /// reports the worst single section so a single
    /// `fail` or `unavailable` is never masked.
    pub fn rollup(&self) -> PortalStatus {
        let mut worst = PortalStatus::Ok;
        for section in &self.sections {
            let rank = match section.status {
                PortalStatus::Ok => 0,
                PortalStatus::Partial => 1,
                PortalStatus::Warn => 2,
                PortalStatus::Unknown => 3,
                PortalStatus::Unavailable => 4,
                PortalStatus::Fail => 5,
            };
            if rank
                > match worst {
                    PortalStatus::Ok => 0,
                    PortalStatus::Partial => 1,
                    PortalStatus::Warn => 2,
                    PortalStatus::Unknown => 3,
                    PortalStatus::Unavailable => 4,
                    PortalStatus::Fail => 5,
                }
            {
                worst = section.status;
            }
        }
        worst
    }
}

impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "Forge Control Plane".to_string(),
            default_scope: PortalScope::Project,
        }
    }
}

impl std::fmt::Display for PortalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}
