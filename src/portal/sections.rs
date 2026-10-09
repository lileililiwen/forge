//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::fleet::FleetFreshness;
use crate::registry::{ProjectRecord, Registry};
use std::collections::BTreeMap;
use std::path::Path;

use super::activity::{
    count_component_receipts, count_files, count_session_files, count_subdirs,
    read_recent_operations,
};
use super::constants::{
    MAX_ENTRIES_PER_VIEW, MAX_ENTRY_ID_LEN, MAX_PORTAL_FLEET_ENTRIES, PORTAL_CONTRACT_VERSION,
    SECTION_AGENTS, SECTION_ANALYTICS, SECTION_COMPONENTS, SECTION_DEPLOYMENTS,
    SECTION_DOCUMENTATION, SECTION_FEATURES, SECTION_POLICIES, SECTION_PROJECTS,
    SECTION_REPOSITORIES, SECTION_SERVERS, SECTION_SETTINGS, SECTION_SPECS, SUPPORTED_SECTIONS,
};
use super::model::{
    FleetProjection, PortalDashboard, PortalEntry, PortalScope, PortalSection, PortalSectionView,
    PortalStatus,
};
use super::render::utc_now;

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

pub(super) fn validate_entries(entries: &[PortalEntry]) -> Result<(), ForgeError> {
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
    build_dashboard_with_fleet(registry, target, None)
}

/// Fleet-wide dashboard with the optional fleet block. The
/// `fleet` argument carries the read-only workspace registry
/// projection when a registry is configured; `None` renders no
/// fleet block.
pub fn build_dashboard_with_fleet(
    registry: &Registry,
    target: Option<&str>,
    fleet: Option<FleetProjection<'_>>,
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
    let sections = build_all_sections(
        registry,
        scope,
        project_id.as_deref(),
        &project_ids,
        &now,
        fleet,
    )?;
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
    build_section_view_with_fleet(registry, target, section, None)
}

/// Build one section view with the optional fleet block.
pub fn build_section_view_with_fleet(
    registry: &Registry,
    target: Option<&str>,
    section: PortalSection,
    fleet: Option<FleetProjection<'_>>,
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
    let mut sections = build_all_sections(
        registry,
        scope,
        project_id.as_deref(),
        &project_ids,
        &now,
        fleet,
    )?;
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
    fleet: Option<FleetProjection<'_>>,
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
        let view = build_section(
            registry,
            &records,
            scope,
            project_id,
            section,
            now,
            fleet.clone(),
        )?;
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
    fleet: Option<FleetProjection<'_>>,
) -> Result<PortalSectionView, ForgeError> {
    let (entries, source) = match section {
        PortalSection::Projects => {
            let mut entries = build_projects_section(records);
            if scope == PortalScope::Fleet {
                entries.extend(fleet_block_entries(fleet, now));
            }
            (entries, "registry")
        }
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

pub(super) fn controls_for(section: PortalSection) -> Vec<String> {
    match section {
        PortalSection::Projects => vec![
            "forge list".to_string(),
            "forge inspect <project>".to_string(),
            "forge doctor <project>".to_string(),
            "forge fleet inspect <ID> (read-only; unmanaged entries can only be inspected)"
                .to_string(),
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

pub(super) fn build_projects_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

/// Fleet block entries for the projects section of a fleet-scope view.
/// An unconfigured registry contributes no block (`None` never
/// reaches here as `Unconfigured` — the caller omits it). A stale
/// registry renders `warn`, an unobservable one `unavailable`; the
/// worst-status roll-up then keeps the section from masking either as
/// `ok`. `unmanaged` entries carry the full normalized projection but
/// the read-only guarantee is named in the meta entry: fleet
/// mirroring never registers or mutates anything.
pub(super) fn fleet_block_entries(
    fleet: Option<FleetProjection<'_>>,
    now: &str,
) -> Vec<PortalEntry> {
    let Some(fleet) = fleet else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    let (report, source_status) = match &fleet {
        FleetProjection::Failed { code, reason } => {
            entries.push(PortalEntry {
                id: "fleet:source".to_string(),
                label: "workspace fleet registry".to_string(),
                status: PortalStatus::Unavailable,
                source: "fleet".to_string(),
                observed_at: now.to_string(),
                evidence: vec![format!("fleet registry unavailable: {code}: {reason}")],
                attributes: BTreeMap::new(),
            });
            return entries;
        }
        FleetProjection::Configured(report) => (
            *report,
            match report.freshness {
                FleetFreshness::Fresh if report.malformed.is_empty() => PortalStatus::Ok,
                FleetFreshness::Fresh | FleetFreshness::Stale => PortalStatus::Warn,
                // A stale or unobservable source is never masked as ok.
                FleetFreshness::Unconfigured => PortalStatus::Unavailable,
            },
        ),
    };
    let mut meta_evidence = Vec::new();
    if let Some(source) = report.source.as_ref() {
        meta_evidence.push(format!("source={source}"));
    }
    meta_evidence.push(format!(
        "freshness={} observed_at={}",
        report.freshness.id(),
        report.observed_at
    ));
    for malformed in report.malformed.iter().take(14) {
        meta_evidence.push(format!(
            "malformed: name={} reason={}",
            malformed.name, malformed.reason
        ));
    }
    if report.malformed.len() > 14 {
        meta_evidence.push(format!(
            "malformed: …{} further",
            report.malformed.len() - 14
        ));
    }
    let mut attributes = BTreeMap::new();
    attributes.insert("freshness".to_string(), report.freshness.id().to_string());
    attributes.insert("entries".to_string(), report.entries.len().to_string());
    attributes.insert("malformed".to_string(), report.malformed.len().to_string());
    if report.entries.len() > MAX_PORTAL_FLEET_ENTRIES {
        attributes.insert(
            "portal_rendered".to_string(),
            format!("{MAX_PORTAL_FLEET_ENTRIES} of {}", report.entries.len()),
        );
    }
    entries.push(PortalEntry {
        id: "fleet:source".to_string(),
        label: "workspace fleet registry".to_string(),
        status: source_status,
        source: "fleet".to_string(),
        observed_at: now.to_string(),
        evidence: meta_evidence,
        attributes,
    });
    let entry_status = match report.freshness {
        FleetFreshness::Stale | FleetFreshness::Unconfigured => PortalStatus::Warn,
        FleetFreshness::Fresh => PortalStatus::Ok,
    };
    for entry in report.entries.iter().take(MAX_PORTAL_FLEET_ENTRIES) {
        let mut attributes = BTreeMap::new();
        for (key, value) in crate::fleet::entry_fields(entry) {
            attributes.insert(key.to_string(), value);
        }
        entries.push(PortalEntry {
            id: format!("fleet:{}", entry.id),
            label: entry.id.clone(),
            status: entry_status,
            source: "fleet".to_string(),
            observed_at: report.observed_at.clone(),
            evidence: Vec::new(),
            attributes,
        });
    }
    entries
}

pub(super) fn build_features_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_components_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_policies_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_specs_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_agents_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_deployments_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_repositories_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_documentation_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_analytics_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_servers_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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

pub(super) fn build_settings_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
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
