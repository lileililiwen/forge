//! Read-only fleet list loaders used by the JSON API fleet
//! projection in `src/api/fleet.rs`.
//!
//! The in-process portal UI (`src/api/ui/`, since removed) used to
//! own this data path; the loader survived in the JSON API so the
//! `/v1/admin/projects` fleet projection could share it. The
//! loader is a thin wrapper around the registry and the portfolio
//! projection, returning typed view structs the JSON API can
//! turn into its `CandidateRow` model.

use std::path::Path;

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::portfolio::{
    filter_query, Confidence, EvidenceStatus, Lifecycle, PortfolioFilter, PortfolioRow,
};
use crate::registry::{OperationEntry, ProjectRecord, Registry};

#[derive(Debug, Clone)]
pub struct FleetRow {
    pub id: String,
    pub profile: String,
    pub last_state: String,
    pub last_at: String,
    pub subdomain: Option<String>,
    /// User-owned lifecycle classification, or `None` when the
    /// project has never been classified. An absent value renders
    /// as `—`, never as a default lifecycle.
    pub lifecycle: Option<Lifecycle>,
    pub confidence: Option<Confidence>,
    pub tags: Vec<String>,
    /// Displayed state of every source system that has an
    /// observation, already downgraded to `stale` where the
    /// freshness bound has passed.
    pub evidence: Vec<(String, EvidenceStatus)>,
}

impl FleetRow {
    /// Cell summary for the evidence column. A project with no
    /// imported observation reads `no evidence`, which is an
    /// honest absence rather than a pass.
    pub fn evidence_summary(&self) -> String {
        if self.evidence.is_empty() {
            return "no evidence".to_string();
        }
        self.evidence
            .iter()
            .map(|(source, status)| format!("{source} {}", status.label()))
            .collect::<Vec<String>>()
            .join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct FleetListView {
    pub rows: Vec<FleetRow>,
    /// The filter that produced this list, echoed back into the
    /// form so the operator can see and change the active filter.
    pub filter: PortfolioFilter,
    /// Query string for the current filter, or empty when unset.
    pub filter_query: String,
}

pub fn load_fleet_list(db_path: &Path) -> Result<FleetListView, ForgeError> {
    load_fleet_list_filtered(db_path, &PortfolioFilter::default())
}

/// Load the fleet list under an optional portfolio filter.
pub fn load_fleet_list_filtered(
    db_path: &Path,
    filter: &PortfolioFilter,
) -> Result<FleetListView, ForgeError> {
    let registry = Registry::open(db_path)?;
    let projects = registry.list()?;
    let recent = registry.recent_operations(per_project_limit(&projects))?;
    let portfolio = registry.portfolio_fleet(filter, chrono::Utc::now())?;

    let mut rows = Vec::with_capacity(projects.len());
    for project in &projects {
        let last = pick_last_publish(&recent, &project.id);
        // A project the active filter excluded has no portfolio
        // row and must not appear in the roster at all.
        let Some(row) = portfolio
            .iter()
            .find(|row| row.profile.project_id == project.id)
        else {
            continue;
        };
        rows.push(fleet_row_from_parts(project, &last, Some(row)));
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(FleetListView {
        rows,
        filter: filter.clone(),
        filter_query: filter_query(filter),
    })
}

fn fleet_row_from_parts(
    project: &ProjectRecord,
    last: &LastPublishRef<'_>,
    portfolio_row: Option<&PortfolioRow>,
) -> FleetRow {
    FleetRow {
        id: project.id.clone(),
        profile: project.profile.clone(),
        last_state: redact_credentials(&last.state()),
        last_at: last.started_at(),
        subdomain: None,
        lifecycle: portfolio_row.and_then(|row| row.profile.lifecycle),
        confidence: portfolio_row.and_then(|row| row.profile.confidence),
        tags: portfolio_row
            .map(|row| row.tags.iter().map(|tag| tag.name.clone()).collect())
            .unwrap_or_default(),
        evidence: portfolio_row
            .map(|row| {
                row.evidence
                    .iter()
                    .map(|state| (state.source_system.clone(), state.status))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn per_project_limit(projects: &[ProjectRecord]) -> usize {
    let n = projects.len().max(1);
    n * 4
}

fn pick_last_publish<'a>(recent: &'a [OperationEntry], project_id: &str) -> LastPublishRef<'a> {
    match recent
        .iter()
        .find(|e| e.project_id == project_id && (e.kind == "publish" || e.kind == "publish.github"))
    {
        Some(op) => LastPublishRef::Some(&op.state, &op.started_at),
        None => LastPublishRef::None,
    }
}

enum LastPublishRef<'a> {
    Some(&'a str, &'a str),
    None,
}

impl<'a> LastPublishRef<'a> {
    fn state(&self) -> String {
        match self {
            LastPublishRef::Some(s, _) => s.to_string(),
            LastPublishRef::None => "—".to_string(),
        }
    }
    fn started_at(&self) -> String {
        match self {
            LastPublishRef::Some(_, t) => t.to_string(),
            LastPublishRef::None => "—".to_string(),
        }
    }
}
