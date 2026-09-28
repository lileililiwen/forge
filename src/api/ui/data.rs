//! Read-only data access for the in-process portal UI.
//!
//! Every loader returns a typed view struct whose fields are
//! owned `String` values (or `Vec<String>` lists). The renderer
//! passes them straight into maud's `{value}` interpolation,
//! which escapes them by default; no `'static` borrowing or
//! `Box::leak` is needed because the page function builds and
//! renders the maud tree in one expression.
//!
//! Secret-shaped strings are passed through
//! `policy::redact_credentials` before they reach the page,
//! mirroring the CLI evidence redaction.

use std::collections::BTreeMap;
use std::path::Path;

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::registry::{OperationEntry, ProjectRecord, Registry};

use super::render::{DoctorKind, DoctorSummary};

// --- fleet list --------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FleetRow {
    pub id: String,
    pub profile: String,
    pub last_state: String,
    pub last_at: String,
    pub subdomain: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FleetListView {
    pub rows: Vec<FleetRow>,
}

pub fn load_fleet_list(db_path: &Path) -> Result<FleetListView, ForgeError> {
    let registry = Registry::open(db_path)?;
    let projects = registry.list()?;
    let recent = registry.recent_operations(per_project_limit(&projects))?;

    let mut rows = Vec::with_capacity(projects.len());
    for project in &projects {
        let last = pick_last_publish(&recent, &project.id);
        rows.push(FleetRow {
            id: project.id.clone(),
            profile: project.profile.clone(),
            last_state: redact_credentials(&last.state()),
            last_at: last.started_at(),
            subdomain: None,
        });
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(FleetListView { rows })
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

// --- project detail ----------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProjectIdentity {
    pub id: String,
    pub profile: String,
    pub maturity: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone)]
pub struct JournalRowView {
    pub started_at: String,
    pub kind: String,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ProjectDetailView {
    pub identity: ProjectIdentity,
    pub doctor: DoctorSummary,
    pub inventory_subdomain: String,
    pub journal: Vec<JournalRowView>,
}

pub fn load_project_detail(
    db_path: &Path,
    project_id: &str,
) -> Result<ProjectDetailView, ForgeError> {
    let registry = Registry::open(db_path)?;
    let record = registry.inspect(project_id)?;
    let operations = registry.operations_for_project(project_id, 20)?;

    let identity = ProjectIdentity {
        id: record.id.clone(),
        profile: record.profile.clone(),
        maturity: record.maturity.clone().unwrap_or_else(|| "—".to_string()),
        manifest_path: record.path.clone(),
    };
    let doctor = doctor_summary_for(&record);
    let journal = operations
        .into_iter()
        .map(|op| JournalRowView {
            started_at: op.started_at.clone(),
            kind: op.kind.clone(),
            state: redact_credentials(&op.state),
            detail: op
                .detail
                .clone()
                .map(|d| redact_credentials(&d))
                .unwrap_or_default(),
        })
        .collect();

    Ok(ProjectDetailView {
        identity,
        doctor,
        inventory_subdomain: String::new(),
        journal,
    })
}

fn doctor_summary_for(record: &ProjectRecord) -> DoctorSummary {
    match record.maturity.as_deref() {
        Some("L3") | Some("L4") => DoctorSummary {
            kind: DoctorKind::Pass,
            label: "verified".to_string(),
            detail: String::new(),
        },
        Some("L2") => DoctorSummary {
            kind: DoctorKind::Warn,
            label: "partial".to_string(),
            detail: "maturity L2".to_string(),
        },
        Some("L1") => DoctorSummary {
            kind: DoctorKind::Warn,
            label: "draft".to_string(),
            detail: "maturity L1".to_string(),
        },
        Some(_) => DoctorSummary {
            kind: DoctorKind::Unknown,
            label: "unknown maturity".to_string(),
            detail: String::new(),
        },
        None => DoctorSummary {
            kind: DoctorKind::Unknown,
            label: "—".to_string(),
            detail: "no maturity recorded".to_string(),
        },
    }
}

// --- publish plan ------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PublishPlanStep {
    pub stage: String,
    pub status: String,
    pub note: String,
}

pub fn load_publish_plan_steps(
    _db_path: &Path,
    _project_id: &str,
) -> Result<Vec<PublishPlanStep>, ForgeError> {
    Ok(vec![
        PublishPlanStep {
            stage: "sync".to_string(),
            status: "dry-run".to_string(),
            note: "rsync the source tree to the target workdir".to_string(),
        },
        PublishPlanStep {
            stage: "db".to_string(),
            status: "dry-run".to_string(),
            note: "render the database overlay".to_string(),
        },
        PublishPlanStep {
            stage: "prepare".to_string(),
            status: "dry-run".to_string(),
            note: "render Compose, router and port documents".to_string(),
        },
        PublishPlanStep {
            stage: "deploy".to_string(),
            status: "dry-run".to_string(),
            note: "ship rendered documents and run the Compose stack".to_string(),
        },
    ])
}

#[derive(Debug, Clone)]
pub struct OperationIdentity {
    pub op_id: String,
    pub revision: String,
    pub container_identity: String,
}

/// Persist a `publish.ui` journal row so the operator can
/// confirm the enqueue through `forge deploy status`.
pub fn record_ui_publish(
    db_path: &Path,
    project_id: &str,
) -> Result<OperationIdentity, ForgeError> {
    let registry = Registry::open(db_path)?;
    let detail = format!("publish.ui enqueued for `{project_id}`");
    let _ = registry.record_operation("publish.ui", project_id, "pending", &detail);
    let op_id = registry
        .recent_operations(64)?
        .into_iter()
        .find(|e| e.kind == "publish.ui" && e.project_id == project_id)
        .map(|e| e.op_id)
        .unwrap_or(0);
    Ok(OperationIdentity {
        op_id: op_id.to_string(),
        revision: "—".to_string(),
        container_identity: "—".to_string(),
    })
}

// --- skipped rows adapter ---------------------------------------------

#[derive(Debug, Clone)]
pub struct SkippedRow {
    pub id: String,
    pub classification: String,
    pub reason: String,
}

/// Adapter from the inventory classifier's skipped-entry
/// representation to the renderer's view struct. Kept here
/// so the loader and the renderer don't have to know about
/// each other's representation.
pub fn skipped_rows_from_pairs(pairs: Vec<(String, String, String)>) -> Vec<SkippedRow> {
    pairs
        .into_iter()
        .map(|(id, classification, reason)| SkippedRow {
            id,
            classification,
            reason,
        })
        .collect()
}

#[allow(dead_code)]
pub fn subdomains_for_projects(db_path: &Path) -> Result<BTreeMap<String, String>, ForgeError> {
    let _ = db_path;
    Ok(BTreeMap::new())
}
