//! Optional control-plane portal data surface
//! (`control-plane-portal`).
//!
//! Forge Core owns the typed portal contract. The portal
//! is a *read-only* view layer that aggregates the data
//! every other Core module already publishes (registry,
//! doctor, features, components, specs, agents, deploys,
//! distribution, docs, analytics, identity) into a single
//! stable, versioned JSON envelope that a future portal
//! framework (ASP.NET Core or Next.js, per the design ADR)
//! can render. Core returns typed outcomes; the portal
//! never reinterprets Core state.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §36 calls for an
//! optional web portal that surfaces the shared control
//! plane. The brief explicitly states the portal should
//! "consume the same Forge Core APIs as CLI and MCP", so
//! the portal is a thin view of the Core registry plus
//! the timestamped observations every other module
//! already journals. No business rule is duplicated: the
//! portal is a renderer, not a reimplementation.
//!
//! ## Boundary
//!
//! - **Read-only projection.** The portal surface never
//!   mutates the manifest, the registry, the feature
//!   receipts, the spec proposals, the deploy state, the
//!   distribution state, the docs derivatives or the
//!   identity sessions. The CLI and the mature MCP tools
//!   remain the only mutating transports; the portal
//!   surfaces a `controls_available` line so the operator
//!   sees where the same operation can be invoked.
//! - **Evidence is timestamped and versioned.** Every
//!   portal view carries the `contract` version, a
//!   `generated_at` timestamp and a `source` field
//!   (registry / doctor / feature / …) so a stale or
//!   unknown observation is never silently re-rendered as
//!   success. A dashboard can imply stronger health than
//!   the evidence supports; the contract surfaces the
//!   `unknown` / `stale` / `partial` states explicitly
//!   (R3 boundary).
//! - **No implicit remote write.** The portal is
//!   loopback-friendly: it never reaches a remote API,
//!   never invokes an external adapter, never authenticates
//!   against an OIDC provider. Authorization is performed
//!   by the underlying Core contract; the portal is just a
//!   viewer.
//! - **Bounded sections.** The portal ships the twelve
//!   named sections listed in §36. Unknown sections are
//!   refused at validation time so the CLI cannot silently
//!   ask for a view the registry cannot answer.
//!
//! ## Persistence
//!
//! No new persistent state is added: the portal reads the
//! manifest, the registry's `projects` and `operations`
//! tables, the doctor report, the per-module state files
//! and the per-module journals. The Core registry's
//! `operations` table receives one `portal` row per
//! `dashboard` / `view` call with a `done` / `partial`
//! verdict and the project id (or the synthetic
//! `__portal__` project id when the view spans the entire
//! registry).
//!
//! ## Risk model
//!
//! A real portal framework is out of scope for the local
//! sandbox: the contract is validated through the
//! `forge portal dashboard` / `forge portal view <section>`
//! CLI surface and the typed `portal-invalid` error. The
//! JSON envelope is the contract a future ASP.NET Core or
//! Next.js renderer would consume. A real portal
//! integration is a downstream step.
#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::registry::{ProjectRecord, Registry};

/// Contract data version for the portal surface. The CLI
/// and the future ASP.NET Core / Next.js renderer speak
/// the same version on the wire.
pub const PORTAL_CONTRACT_VERSION: &str = "0.1.0";

/// Synthetic project id recorded against the `portal`
/// journal when the view spans the entire registered
/// registry (e.g. `forge portal dashboard --all`). The id
/// keeps the operations table project-agnostic without
/// inventing a user-visible project.
pub const PORTAL_SYNTHETIC_PROJECT: &str = "__portal__";

/// Section id parsing. The brief lists twelve named
/// sections in §36; the contract refuses unknown ids so
/// the CLI cannot silently ask for a view the registry
/// cannot answer.
pub const SECTION_PROJECTS: &str = "projects";
pub const SECTION_FEATURES: &str = "features";
pub const SECTION_COMPONENTS: &str = "components";
pub const SECTION_POLICIES: &str = "policies";
pub const SECTION_SPECS: &str = "specs";
pub const SECTION_AGENTS: &str = "agents";
pub const SECTION_DEPLOYMENTS: &str = "deployments";
pub const SECTION_REPOSITORIES: &str = "repositories";
pub const SECTION_DOCUMENTATION: &str = "documentation";
pub const SECTION_ANALYTICS: &str = "analytics";
pub const SECTION_SERVERS: &str = "servers";
pub const SECTION_SETTINGS: &str = "settings";

/// Stable list of supported section ids, in the order the
/// brief enumerates them in §36. The order is preserved by
/// `parse_section` and by the human renderer.
pub const SUPPORTED_SECTIONS: &[&str] = &[
    SECTION_PROJECTS,
    SECTION_FEATURES,
    SECTION_COMPONENTS,
    SECTION_POLICIES,
    SECTION_SPECS,
    SECTION_AGENTS,
    SECTION_DEPLOYMENTS,
    SECTION_REPOSITORIES,
    SECTION_DOCUMENTATION,
    SECTION_ANALYTICS,
    SECTION_SERVERS,
    SECTION_SETTINGS,
];

/// Maximum number of section entries a single portal view
/// may carry. Larger lists are refused at validation time
/// so a malformed manifest or a runaway registry cannot
/// pin the renderer to an unbounded allocation.
pub const MAX_ENTRIES_PER_VIEW: usize = 256;

/// Maximum length of an entry `id` (a project id, a
/// feature id, …). Longer ids are refused at validation
/// time so a malicious input cannot inject oversized
/// text.
pub const MAX_ENTRY_ID_LEN: usize = 128;

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

/// Parse a kebab-case section id. Unknown ids are refused
/// with the typed `portal-invalid` code so the CLI cannot
/// silently ask for a view the registry cannot answer.
pub fn parse_section(raw: &str) -> Result<PortalSection, ForgeError> {
    let trimmed = raw.trim();
    if !SUPPORTED_SECTIONS.contains(&trimmed) {
        return Err(ForgeError::PortalInvalid {
            reason: format!(
                "unknown portal section `{trimmed}`; supported: {}",
                SUPPORTED_SECTIONS.join(", ")
            ),
        });
    }
    let section = match trimmed {
        SECTION_PROJECTS => PortalSection::Projects,
        SECTION_FEATURES => PortalSection::Features,
        SECTION_COMPONENTS => PortalSection::Components,
        SECTION_POLICIES => PortalSection::Policies,
        SECTION_SPECS => PortalSection::Specs,
        SECTION_AGENTS => PortalSection::Agents,
        SECTION_DEPLOYMENTS => PortalSection::Deployments,
        SECTION_REPOSITORIES => PortalSection::Repositories,
        SECTION_DOCUMENTATION => PortalSection::Documentation,
        SECTION_ANALYTICS => PortalSection::Analytics,
        SECTION_SERVERS => PortalSection::Servers,
        SECTION_SETTINGS => PortalSection::Settings,
        _ => unreachable!("SUPPORTED_SECTIONS guarded the match above"),
    };
    Ok(section)
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

impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "Forge Control Plane".to_string(),
            default_scope: PortalScope::Project,
        }
    }
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

impl std::fmt::Display for PortalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Bounded entry validator. Refuses empty id, oversized
/// id, oversized evidence and oversized attribute maps so
/// the renderer never crashes on a malicious input.
fn validate_entry(entry: &PortalEntry) -> Result<(), ForgeError> {
    if entry.id.trim().is_empty() {
        return Err(ForgeError::PortalInvalid {
            reason: "portal entry `id` is empty".to_string(),
        });
    }
    if entry.id.len() > MAX_ENTRY_ID_LEN {
        return Err(ForgeError::PortalInvalid {
            reason: format!(
                "portal entry `id` is {len} chars; max {MAX_ENTRY_ID_LEN}",
                len = entry.id.len()
            ),
        });
    }
    if entry.evidence.len() > 16 {
        return Err(ForgeError::PortalInvalid {
            reason: format!(
                "portal entry `{}` carries {} evidence lines; max 16",
                entry.id,
                entry.evidence.len()
            ),
        });
    }
    if entry.attributes.len() > 16 {
        return Err(ForgeError::PortalInvalid {
            reason: format!(
                "portal entry `{}` carries {} attributes; max 16",
                entry.id,
                entry.attributes.len()
            ),
        });
    }
    Ok(())
}

fn validate_entries(entries: &[PortalEntry]) -> Result<(), ForgeError> {
    if entries.len() > MAX_ENTRIES_PER_VIEW {
        return Err(ForgeError::PortalInvalid {
            reason: format!(
                "portal view carries {n} entries; max {MAX_ENTRIES_PER_VIEW}",
                n = entries.len()
            ),
        });
    }
    for entry in entries {
        validate_entry(entry)?;
    }
    Ok(())
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
    fn rollup_status(entries: &[PortalEntry]) -> PortalStatus {
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

/// Build the per-project dashboard. The `target`
/// argument is either a registered project id (a
/// `ProjectRecord.path` lookup) or `None` for the
/// fleet-wide dashboard. The dashboard never mutates
/// the registry; the only write is a single `portal`
/// journal row recorded by the caller.
pub fn build_dashboard(
    registry: &Registry,
    target: Option<&str>,
) -> Result<PortalDashboard, ForgeError> {
    let now = utc_now();
    let title = "Forge Control Plane".to_string();
    let (scope, project_id, project_ids) = match target {
        Some(query) => {
            let record = registry.inspect(query)?;
            (
                PortalScope::Project,
                Some(record.id.clone()),
                vec![record.id],
            )
        }
        None => {
            let records = registry.list()?;
            let ids = records.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
            (PortalScope::Fleet, None, ids)
        }
    };
    let sections = build_all_sections(registry, scope, project_id.as_deref(), &project_ids, &now)?;
    let operations = read_recent_operations(registry, project_id.as_deref(), 8)?;
    let dashboard = PortalDashboard {
        contract: PORTAL_CONTRACT_VERSION.to_string(),
        generated_at: now,
        title,
        scope,
        project_id: project_id.clone(),
        section_count: sections.len(),
        sections,
        operations,
    };
    Ok(dashboard)
}

/// Build one section view. The function is the
/// single-section counterpart of `build_dashboard` and
/// shares the same evidence pipeline.
pub fn build_section_view(
    registry: &Registry,
    target: Option<&str>,
    section: PortalSection,
) -> Result<PortalSectionView, ForgeError> {
    let now = utc_now();
    let (scope, project_id, project_ids) = match target {
        Some(query) => {
            let record = registry.inspect(query)?;
            (
                PortalScope::Project,
                Some(record.id.clone()),
                vec![record.id],
            )
        }
        None => {
            let records = registry.list()?;
            let ids = records.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
            (PortalScope::Fleet, None, ids)
        }
    };
    let mut sections =
        build_all_sections(registry, scope, project_id.as_deref(), &project_ids, &now)?;
    let view = sections
        .drain(..)
        .find(|s| s.section == section)
        .ok_or_else(|| ForgeError::PortalInvalid {
            reason: format!("section `{}` not present in dashboard", section.id()),
        })?;
    Ok(view)
}

fn build_all_sections(
    registry: &Registry,
    scope: PortalScope,
    project_id: Option<&str>,
    project_ids: &[String],
    now: &str,
) -> Result<Vec<PortalSectionView>, ForgeError> {
    let records = resolve_records(registry, project_id, project_ids)?;
    let mut sections = Vec::with_capacity(SUPPORTED_SECTIONS.len());
    for section in [
        PortalSection::Projects,
        PortalSection::Features,
        PortalSection::Components,
        PortalSection::Policies,
        PortalSection::Specs,
        PortalSection::Agents,
        PortalSection::Deployments,
        PortalSection::Repositories,
        PortalSection::Documentation,
        PortalSection::Analytics,
        PortalSection::Servers,
        PortalSection::Settings,
    ] {
        let view = build_section(registry, &records, scope, project_id, section, now)?;
        sections.push(view);
    }
    Ok(sections)
}

fn resolve_records(
    registry: &Registry,
    project_id: Option<&str>,
    project_ids: &[String],
) -> Result<Vec<ProjectRecord>, ForgeError> {
    if let Some(id) = project_id {
        let record = registry.inspect(id)?;
        return Ok(vec![record]);
    }
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(project_ids.len());
    for id in project_ids {
        out.push(registry.inspect(id)?);
    }
    Ok(out)
}

fn build_section(
    _registry: &Registry,
    records: &[ProjectRecord],
    scope: PortalScope,
    project_id: Option<&str>,
    section: PortalSection,
    now: &str,
) -> Result<PortalSectionView, ForgeError> {
    let (entries, source) = match section {
        PortalSection::Projects => (build_projects_section(records), "registry"),
        PortalSection::Features => (build_features_section(records), "registry"),
        PortalSection::Components => (build_components_section(records), "registry"),
        PortalSection::Policies => (build_policies_section(records), "registry"),
        PortalSection::Specs => (build_specs_section(records), "registry"),
        PortalSection::Agents => (build_agents_section(records), "registry"),
        PortalSection::Deployments => (build_deployments_section(records), "registry"),
        PortalSection::Repositories => (build_repositories_section(records), "registry"),
        PortalSection::Documentation => (build_documentation_section(records), "registry"),
        PortalSection::Analytics => (build_analytics_section(records), "registry"),
        PortalSection::Servers => (build_servers_section(records), "registry"),
        PortalSection::Settings => (build_settings_section(records), "registry"),
    };
    validate_entries(&entries)?;
    let status = PortalSectionView::rollup_status(&entries);
    let view = PortalSectionView {
        contract: PORTAL_CONTRACT_VERSION.to_string(),
        generated_at: now.to_string(),
        project_id: project_id.map(str::to_string),
        scope,
        section,
        section_id: section.id().to_string(),
        status,
        source: source.to_string(),
        entries,
        controls_available: controls_for(section),
    };
    Ok(view)
}

fn controls_for(section: PortalSection) -> Vec<String> {
    match section {
        PortalSection::Projects => vec![
            "forge list".to_string(),
            "forge inspect <project>".to_string(),
            "forge doctor <project>".to_string(),
        ],
        PortalSection::Features => vec![
            "forge feature list".to_string(),
            "forge feature add <feature>".to_string(),
        ],
        PortalSection::Components => vec!["forge component list".to_string()],
        PortalSection::Policies => vec!["forge doctor".to_string()],
        PortalSection::Specs => vec![
            "forge spec list".to_string(),
            "forge spec generate".to_string(),
        ],
        PortalSection::Agents => vec![
            "forge agent list".to_string(),
            "forge agent start".to_string(),
        ],
        PortalSection::Deployments => vec![
            "forge deploy list".to_string(),
            "forge deploy plan".to_string(),
        ],
        PortalSection::Repositories => vec!["forge mirror".to_string()],
        PortalSection::Documentation => vec!["forge docs translate".to_string()],
        PortalSection::Analytics => vec![
            "forge analytics inspect".to_string(),
            "forge analytics metrics".to_string(),
        ],
        PortalSection::Servers => vec!["forge api serve".to_string()],
        PortalSection::Settings => vec![
            "forge portal dashboard".to_string(),
            "forge governance status".to_string(),
            "forge governance use".to_string(),
        ],
    }
}

fn build_projects_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let status = if !r.available {
                PortalStatus::Unavailable
            } else if r.runtime.is_none() {
                PortalStatus::Unknown
            } else if r.quality_status.as_deref() == Some("fail") {
                PortalStatus::Fail
            } else if r.quality_status.as_deref() == Some("warn") {
                PortalStatus::Warn
            } else {
                PortalStatus::Ok
            };
            let mut attributes = BTreeMap::new();
            attributes.insert("profile".to_string(), r.profile.clone());
            attributes.insert(
                "maturity".to_string(),
                r.maturity.clone().unwrap_or_else(|| "unknown".to_string()),
            );
            attributes.insert(
                "target_maturity".to_string(),
                r.target_maturity
                    .clone()
                    .unwrap_or_else(|| "L1".to_string()),
            );
            if let Some(target) = r.deployment_target.as_ref() {
                attributes.insert("deployment_target".to_string(), target.clone());
            }
            PortalEntry {
                id: r.id.clone(),
                label: r.name.clone(),
                status,
                source: "registry".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: Vec::new(),
                attributes,
            }
        })
        .collect()
}

fn build_features_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    let mut entries = Vec::new();
    for r in records {
        if r.features.is_empty() {
            continue;
        }
        for (feature, version) in &r.features {
            entries.push(PortalEntry {
                id: format!("{r_id}:{feature}", r_id = r.id),
                label: feature.clone(),
                status: PortalStatus::Ok,
                source: "feature".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("version={version}")],
                attributes: BTreeMap::new(),
            });
        }
    }
    entries
}

fn build_components_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    let mut entries = Vec::new();
    for r in records {
        let path = std::path::Path::new(&r.path).join(".forge/components");
        let count = count_component_receipts(&path);
        let status = if count == 0 {
            PortalStatus::Unknown
        } else {
            PortalStatus::Ok
        };
        entries.push(PortalEntry {
            id: format!("{r_id}:components", r_id = r.id),
            label: format!("{} components", r.name),
            status,
            source: "component".to_string(),
            observed_at: r.observed_at.clone(),
            evidence: vec![format!("receipts={count}")],
            attributes: BTreeMap::new(),
        });
    }
    entries
}

fn build_policies_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let status = match r.quality_status.as_deref() {
                Some("ok") | Some("healthy") | None if r.runtime.is_some() => PortalStatus::Ok,
                Some("warn") => PortalStatus::Warn,
                Some("fail") => PortalStatus::Fail,
                _ => PortalStatus::Unknown,
            };
            PortalEntry {
                id: format!("{r_id}:policies", r_id = r.id),
                label: format!("{} driftwatch policies", r.name),
                status,
                source: "policy".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_specs_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let path = std::path::Path::new(&r.path).join(".forge/specs");
            let count = count_subdirs(&path);
            let status = if count == 0 {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            PortalEntry {
                id: format!("{r_id}:specs", r_id = r.id),
                label: format!("{} spec proposals", r.name),
                status,
                source: "spec".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("proposals={count}")],
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_agents_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let path = std::path::Path::new(&r.path).join(".forge/agents");
            let count = count_subdirs(&path);
            let status = if count == 0 {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            PortalEntry {
                id: format!("{r_id}:agents", r_id = r.id),
                label: format!("{} agent sessions", r.name),
                status,
                source: "agent".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("sessions={count}")],
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_deployments_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let path = std::path::Path::new(&r.path).join(".forge/deploy");
            let count = count_subdirs(&path);
            let status = if count == 0 {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            PortalEntry {
                id: format!("{r_id}:deployments", r_id = r.id),
                label: format!("{} deploys", r.name),
                status,
                source: "deploy".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("states={count}")],
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_repositories_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let status = if r.git_remote.is_none() {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            let mut attributes = BTreeMap::new();
            if let Some(remote) = r.git_remote.as_ref() {
                attributes.insert("primary".to_string(), remote.clone());
            }
            if !r.mirror_remotes.is_empty() {
                attributes.insert("mirrors".to_string(), r.mirror_remotes.join(","));
            }
            PortalEntry {
                id: format!("{r_id}:repositories", r_id = r.id),
                label: format!("{} repositories", r.name),
                status,
                source: "distribution".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: Vec::new(),
                attributes,
            }
        })
        .collect()
}

fn build_documentation_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let status = match r.docs_status.as_deref() {
                Some("ok") | Some("healthy") => PortalStatus::Ok,
                Some("warn") => PortalStatus::Warn,
                Some("fail") => PortalStatus::Fail,
                _ => PortalStatus::Unknown,
            };
            PortalEntry {
                id: format!("{r_id}:documentation", r_id = r.id),
                label: format!("{} documentation", r.name),
                status,
                source: "docs".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_analytics_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let path = std::path::Path::new(&r.path).join(".forge/analytics");
            let count = count_files(&path);
            let status = if count == 0 {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            PortalEntry {
                id: format!("{r_id}:analytics", r_id = r.id),
                label: format!("{} analytics summaries", r.name),
                status,
                source: "analytics".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("summaries={count}")],
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_servers_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let path = std::path::Path::new(&r.path).join(".forge/identity");
            let count = count_session_files(&path);
            let status = if count == 0 {
                PortalStatus::Unknown
            } else {
                PortalStatus::Ok
            };
            PortalEntry {
                id: format!("{r_id}:servers", r_id = r.id),
                label: format!("{} api/identity state", r.name),
                status,
                source: "identity".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: vec![format!("sessions={count}")],
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}

fn build_settings_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let governance = crate::governance::evaluate_project(Path::new(&r.path));
            let (status, evidence, provider) = match governance {
                Ok(observation) => {
                    let status = match observation.status {
                        crate::governance::ProviderStatus::Pass => PortalStatus::Ok,
                        crate::governance::ProviderStatus::Fail => PortalStatus::Fail,
                        crate::governance::ProviderStatus::Blocked => PortalStatus::Warn,
                        crate::governance::ProviderStatus::Unknown
                        | crate::governance::ProviderStatus::Stale => PortalStatus::Unknown,
                        crate::governance::ProviderStatus::Unavailable
                        | crate::governance::ProviderStatus::Disabled
                        | crate::governance::ProviderStatus::Incompatible => {
                            PortalStatus::Unavailable
                        }
                    };
                    (
                        status,
                        vec![format!("governance status: {:?}", observation.status)],
                        observation.provider,
                    )
                }
                Err(err) => (
                    PortalStatus::Unavailable,
                    vec![format!("governance unavailable: {err}")],
                    "local".to_string(),
                ),
            };
            let mut attributes = BTreeMap::new();
            attributes.insert("governance_provider".to_string(), provider);
            PortalEntry {
                id: format!("{r_id}:settings", r_id = r.id),
                label: format!("{} portal and governance settings", r.name),
                status,
                source: "governance".to_string(),
                observed_at: r.observed_at.clone(),
                evidence,
                attributes,
            }
        })
        .collect()
}

fn count_subdirs(path: &Path) -> usize {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .count()
}

fn count_files(path: &Path) -> usize {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .count()
}

fn count_component_receipts(path: &Path) -> usize {
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

fn count_session_files(path: &Path) -> usize {
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

fn read_recent_operations(
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

/// Build a `forge portal view <section> [target]` journal
/// detail line. The renderer prints the section heading,
/// the rolled-up status and one line per entry.
pub fn render_section_human(view: &PortalSectionView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "[{label}] section=`{id}` status=`{status}` source=`{source}` generated_at {ts}\n",
        label = view.section.label(),
        id = view.section_id,
        status = view.status.id(),
        source = view.source,
        ts = view.generated_at,
    ));
    if let Some(id) = view.project_id.as_ref() {
        out.push_str(&format!("  project_id: {id}\n"));
    }
    if view.entries.is_empty() {
        out.push_str("  entries: (none)\n");
    } else {
        out.push_str(&format!("  entries: {n}\n", n = view.entries.len()));
        for entry in &view.entries {
            out.push_str(&format!(
                "  - id=`{id}` status=`{status}` label=`{label}` source=`{source}` observed_at {ts}\n",
                id = entry.id,
                status = entry.status.id(),
                label = entry.label,
                source = entry.source,
                ts = entry.observed_at,
            ));
            for line in &entry.evidence {
                out.push_str(&format!("      evidence: {line}\n"));
            }
            for (key, value) in &entry.attributes {
                out.push_str(&format!("      attr: {key}={value}\n"));
            }
        }
    }
    if !view.controls_available.is_empty() {
        out.push_str(&format!(
            "  controls_available: {}\n",
            view.controls_available.join(", ")
        ));
    }
    out
}

/// RFC 3339 timestamp for the `generated_at` field. The
/// clock is the system clock; the value is stable across
/// every entry in a single view because the caller
/// threads the same `now` through the build pipeline.
pub fn utc_now() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> Manifest {
        let raw = br#"
schema: 1
project:
  id: portal-sample
  name: Portal Sample
  profile: rust-web
  maturity: L2
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: Custom Dashboard
  default_scope: fleet
"#;
        Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap()
    }

    fn empty_meta() -> Manifest {
        let raw = br#"
schema: 1
project:
  id: portal-empty
  name: Empty
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
"#;
        Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap()
    }

    #[test]
    fn parse_section_accepts_every_supported_id() {
        for id in SUPPORTED_SECTIONS {
            let parsed = parse_section(id).expect(id);
            assert_eq!(parsed.id(), *id);
        }
    }

    #[test]
    fn parse_section_refuses_unknown_id() {
        let err = parse_section("unknown").unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn parse_section_is_idempotent_on_whitespace() {
        let parsed = parse_section("  projects  ").unwrap();
        assert_eq!(parsed, PortalSection::Projects);
    }

    #[test]
    fn portal_config_default_is_consistent() {
        let cfg = PortalConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.title, "Forge Control Plane");
        assert_eq!(cfg.default_scope, PortalScope::Project);
    }

    #[test]
    fn portal_config_from_manifest_uses_defaults_when_block_missing() {
        let cfg = PortalConfig::from_manifest(&empty_meta()).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.default_scope, PortalScope::Project);
        assert_eq!(cfg.title, "Forge Control Plane");
    }

    #[test]
    fn portal_config_from_manifest_uses_typed_fields() {
        let cfg = PortalConfig::from_manifest(&sample_meta()).unwrap();
        assert_eq!(cfg.title, "Custom Dashboard");
        assert_eq!(cfg.default_scope, PortalScope::Fleet);
    }

    #[test]
    fn portal_config_from_manifest_refuses_empty_title() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: ""
  default_scope: project
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_oversized_title() {
        let oversized = "a".repeat(129);
        let raw = format!(
            r#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {{}}
portal:
  enabled: true
  title: "{oversized}"
  default_scope: project
"#
        );
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw.as_bytes()).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_unknown_scope() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: true
  title: "Forge"
  default_scope: organization
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn portal_config_from_manifest_refuses_title_with_disabled_block() {
        let raw = br#"
schema: 1
project:
  id: portal-bad
  name: Bad
  profile: rust-web
  maturity: L1
manifest_version: 1
schema_version: 1
platform_version: 0.1.0
features: {}
portal:
  enabled: false
  title: "Custom"
"#;
        let manifest = Manifest::parse(std::path::Path::new("forge.yaml"), raw).unwrap();
        let err = PortalConfig::from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_oversized_list() {
        let entries: Vec<PortalEntry> = (0..(MAX_ENTRIES_PER_VIEW + 1))
            .map(|i| PortalEntry {
                id: format!("e{i}"),
                label: "x".to_string(),
                status: PortalStatus::Ok,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            })
            .collect();
        let err = validate_entries(&entries).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_empty_id() {
        let entry = PortalEntry {
            id: String::new(),
            label: "x".to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            evidence: Vec::new(),
            attributes: BTreeMap::new(),
        };
        let err = validate_entries(&[entry]).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn validate_entries_refuses_oversized_id() {
        let entry = PortalEntry {
            id: "x".repeat(MAX_ENTRY_ID_LEN + 1),
            label: "x".to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            evidence: Vec::new(),
            attributes: BTreeMap::new(),
        };
        let err = validate_entries(&[entry]).unwrap_err();
        assert_eq!(err.code(), "portal-invalid");
    }

    #[test]
    fn section_rollup_returns_worst_status() {
        let entries = vec![
            PortalEntry {
                id: "a".to_string(),
                label: "a".to_string(),
                status: PortalStatus::Ok,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            },
            PortalEntry {
                id: "b".to_string(),
                label: "b".to_string(),
                status: PortalStatus::Fail,
                source: "registry".to_string(),
                observed_at: "2026-01-15T12:00:00Z".to_string(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            },
        ];
        assert_eq!(
            PortalSectionView::rollup_status(&entries),
            PortalStatus::Fail
        );
    }

    #[test]
    fn dashboard_rollup_returns_worst_section() {
        let section_a = PortalSectionView {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            project_id: None,
            scope: PortalScope::Fleet,
            section: PortalSection::Projects,
            section_id: SECTION_PROJECTS.to_string(),
            status: PortalStatus::Ok,
            source: "registry".to_string(),
            entries: Vec::new(),
            controls_available: Vec::new(),
        };
        let section_b = PortalSectionView {
            status: PortalStatus::Unavailable,
            ..section_a.clone()
        };
        let dashboard = PortalDashboard {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            title: "Forge".to_string(),
            scope: PortalScope::Fleet,
            project_id: None,
            section_count: 2,
            sections: vec![section_a, section_b],
            operations: Vec::new(),
        };
        assert_eq!(dashboard.rollup(), PortalStatus::Unavailable);
    }

    #[test]
    fn dashboard_drops_project_id_for_fleet_scope() {
        let records: Vec<ProjectRecord> = Vec::new();
        let dashboard = build_dashboard_from_records(
            &records,
            PortalScope::Fleet,
            None,
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        assert!(dashboard.project_id.is_none());
        assert_eq!(dashboard.scope, PortalScope::Fleet);
        assert_eq!(dashboard.sections.len(), SUPPORTED_SECTIONS.len());
    }

    fn build_dashboard_from_records(
        records: &[ProjectRecord],
        scope: PortalScope,
        project_id: Option<&str>,
        now: &str,
    ) -> Result<PortalDashboard, ForgeError> {
        let mut sections = Vec::new();
        for section in [
            PortalSection::Projects,
            PortalSection::Features,
            PortalSection::Components,
            PortalSection::Policies,
            PortalSection::Specs,
            PortalSection::Agents,
            PortalSection::Deployments,
            PortalSection::Repositories,
            PortalSection::Documentation,
            PortalSection::Analytics,
            PortalSection::Servers,
            PortalSection::Settings,
        ] {
            let (entries, source) = match section {
                PortalSection::Projects => (build_projects_section(records), "registry"),
                PortalSection::Features => (build_features_section(records), "registry"),
                PortalSection::Components => (build_components_section(records), "registry"),
                PortalSection::Policies => (build_policies_section(records), "registry"),
                PortalSection::Specs => (build_specs_section(records), "registry"),
                PortalSection::Agents => (build_agents_section(records), "registry"),
                PortalSection::Deployments => (build_deployments_section(records), "registry"),
                PortalSection::Repositories => (build_repositories_section(records), "registry"),
                PortalSection::Documentation => (build_documentation_section(records), "registry"),
                PortalSection::Analytics => (build_analytics_section(records), "registry"),
                PortalSection::Servers => (build_servers_section(records), "registry"),
                PortalSection::Settings => (build_settings_section(records), "registry"),
            };
            validate_entries(&entries)?;
            let status = PortalSectionView::rollup_status(&entries);
            sections.push(PortalSectionView {
                contract: PORTAL_CONTRACT_VERSION.to_string(),
                generated_at: now.to_string(),
                project_id: project_id.map(str::to_string),
                scope,
                section,
                section_id: section.id().to_string(),
                status,
                source: source.to_string(),
                entries,
                controls_available: controls_for(section),
            });
        }
        Ok(PortalDashboard {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: now.to_string(),
            title: "Forge Control Plane".to_string(),
            scope,
            project_id: project_id.map(str::to_string),
            section_count: sections.len(),
            sections,
            operations: Vec::new(),
        })
    }

    #[test]
    fn dashboard_uses_unavailable_for_missing_projects() {
        let record = ProjectRecord {
            id: "p1".to_string(),
            name: "P1".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::new(),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: None,
            agent_status: None,
            docs_status: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: false,
        };
        let dashboard = build_dashboard_from_records(
            &[record],
            PortalScope::Project,
            Some("p1"),
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        let projects_section = dashboard
            .sections
            .iter()
            .find(|s| s.section == PortalSection::Projects)
            .unwrap();
        assert_eq!(projects_section.status, PortalStatus::Unavailable);
    }

    #[test]
    fn human_renderer_carries_required_fields() {
        let dashboard =
            build_dashboard_from_records(&[], PortalScope::Fleet, None, "2026-01-15T12:00:00Z")
                .unwrap();
        let text = render_dashboard_human(&dashboard);
        for needle in [
            PORTAL_CONTRACT_VERSION,
            "Forge Control Plane",
            "fleet",
            "rollup",
            "sections",
            "recent operations",
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
    }

    #[test]
    fn section_human_renderer_carries_required_fields() {
        let record = ProjectRecord {
            id: "p1".to_string(),
            name: "P1".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::from([("auth".to_string(), "0.1.0".to_string())]),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: Some("warn".to_string()),
            agent_status: None,
            docs_status: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: true,
        };
        let mut features = build_features_section(std::slice::from_ref(&record));
        assert_eq!(features.len(), 1);
        let entry = features.remove(0);
        assert_eq!(entry.id, "p1:auth");
        assert_eq!(entry.status, PortalStatus::Ok);
        let view = PortalSectionView {
            contract: PORTAL_CONTRACT_VERSION.to_string(),
            generated_at: "2026-01-15T12:00:00Z".to_string(),
            project_id: Some("p1".to_string()),
            scope: PortalScope::Project,
            section: PortalSection::Features,
            section_id: SECTION_FEATURES.to_string(),
            status: PortalStatus::Ok,
            source: "feature".to_string(),
            entries: vec![entry],
            controls_available: controls_for(PortalSection::Features),
        };
        let text = render_section_human(&view);
        for needle in [
            PORTAL_CONTRACT_VERSION,
            "Features",
            SECTION_FEATURES,
            "p1:auth",
            "version=0.1.0",
            "controls_available",
        ] {
            assert!(text.contains(needle), "{needle} missing in {text}");
        }
    }

    #[test]
    fn section_view_returns_requested_section_only() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("registry.db");
        let registry = Registry::open(&db_path).unwrap();
        // No registered projects: the dashboard is built against
        // an empty registry; the section view is the same view
        // for the requested section.
        let view = build_section_view(&registry, None, PortalSection::Servers).unwrap();
        assert_eq!(view.section, PortalSection::Servers);
        assert_eq!(view.section_id, SECTION_SERVERS);
        assert!(view.entries.is_empty());
    }

    #[test]
    fn dashboard_records_unknown_observations_prominently() {
        let record = ProjectRecord {
            id: "p-unknown".to_string(),
            name: "Unknown".to_string(),
            path: "/nonexistent".to_string(),
            git_remote: None,
            mirror_remotes: Vec::new(),
            stack: None,
            profile: "rust-web".to_string(),
            maturity: Some("L1".to_string()),
            target_maturity: Some("L2".to_string()),
            schema_version: 1,
            platform_version: "0.1.0".to_string(),
            features: BTreeMap::new(),
            deployment_target: None,
            runtime: None,
            last_commit: None,
            quality_status: None,
            agent_status: None,
            docs_status: None,
            observed_at: "2026-01-15T12:00:00Z".to_string(),
            available: true,
        };
        let dashboard = build_dashboard_from_records(
            &[record],
            PortalScope::Project,
            Some("p-unknown"),
            "2026-01-15T12:00:00Z",
        )
        .unwrap();
        // A project with no runtime/quality/agents/... is
        // reported as `unknown`, not `ok`. The dashboard rollup
        // surfaces the worst section so the operator never sees
        // a false-positive `ok`.
        let rollup = dashboard.rollup();
        assert!(
            matches!(
                rollup,
                PortalStatus::Unknown | PortalStatus::Unavailable | PortalStatus::Warn
            ),
            "rollup was {rollup:?}"
        );
    }

    #[test]
    fn portal_section_label_matches_id() {
        for section in [
            PortalSection::Projects,
            PortalSection::Features,
            PortalSection::Components,
            PortalSection::Policies,
            PortalSection::Specs,
            PortalSection::Agents,
            PortalSection::Deployments,
            PortalSection::Repositories,
            PortalSection::Documentation,
            PortalSection::Analytics,
            PortalSection::Servers,
            PortalSection::Settings,
        ] {
            assert!(!section.id().is_empty());
            assert!(!section.label().is_empty());
            assert!(SUPPORTED_SECTIONS.contains(&section.id()));
        }
    }
}
