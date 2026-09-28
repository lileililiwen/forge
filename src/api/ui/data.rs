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
use crate::portfolio::{
    filter_query, Confidence, EvidenceStatus, Lifecycle, PortfolioFilter, PortfolioRow,
};
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

/// Load the fleet list under an optional portfolio filter. The
/// filter combines user-owned fields (lifecycle, confidence, tags)
/// with the displayed evidence state; it never widens the result
/// set silently — an out-of-vocabulary value is refused by
/// [`crate::portfolio::parse_filter`] before this function runs.
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
    pub portfolio: ProjectPortfolioView,
}

/// The portfolio half of one project detail page: user-owned
/// classification, tags, goals, relations, imported evidence and
/// review history. Every field is read-only here — the write path
/// is the confirm-gated `POST /ui/projects/{id}/portfolio`.
#[derive(Debug, Clone)]
pub struct ProjectPortfolioView {
    pub lifecycle: String,
    pub confidence: String,
    pub next_action: String,
    pub blocker: String,
    pub reviewed_at: String,
    pub tags: Vec<String>,
    pub goals: Vec<(String, String)>,
    pub relations: Vec<PortfolioRelationView>,
    pub evidence: Vec<EvidenceRowView>,
    pub reviews: Vec<ReviewRowView>,
}

/// One relation rendered on the detail page, labelled with the
/// direction it was declared in and a link to the other project.
#[derive(Debug, Clone)]
pub struct PortfolioRelationView {
    pub direction: String,
    pub other_project: String,
    pub relation_type: String,
    pub note: String,
}

/// One imported source-owned observation. `source_system`,
/// `source_revision` and `observed_at` are rendered verbatim so
/// the page attributes the observation to its source instead of
/// implying Forge performed the check.
#[derive(Debug, Clone)]
pub struct EvidenceRowView {
    pub source_system: String,
    pub source_revision: String,
    pub observed_at: String,
    pub status: EvidenceStatus,
    pub stale_after: String,
}

/// One recorded review decision.
#[derive(Debug, Clone)]
pub struct ReviewRowView {
    pub reviewed_at: String,
    pub confidence: String,
    pub note: String,
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
        portfolio: load_project_portfolio(db_path, project_id)?,
    })
}

/// Load one project's portfolio projection for the detail page.
/// An empty projection is still a truthful one: an unclassified
/// project with no imported evidence renders every field as `—`
/// rather than omitting the section.
pub fn load_project_portfolio(
    db_path: &Path,
    project_id: &str,
) -> Result<ProjectPortfolioView, ForgeError> {
    let registry = Registry::open(db_path)?;
    let view = registry.portfolio_project_view(project_id, chrono::Utc::now())?;
    Ok(ProjectPortfolioView {
        lifecycle: view
            .profile
            .lifecycle
            .map(|v| v.label().to_string())
            .unwrap_or_else(|| "—".to_string()),
        confidence: view
            .profile
            .confidence
            .map(|v| v.label().to_string())
            .unwrap_or_else(|| "—".to_string()),
        next_action: view.profile.next_action.unwrap_or_else(|| "—".to_string()),
        blocker: view.profile.blocker.unwrap_or_else(|| "—".to_string()),
        reviewed_at: view.profile.reviewed_at.unwrap_or_else(|| "—".to_string()),
        tags: view.tags.into_iter().map(|tag| tag.name).collect(),
        goals: view
            .goals
            .into_iter()
            .map(|goal| (goal.title, goal.status))
            .collect(),
        relations: view
            .relations
            .into_iter()
            .map(|relation| PortfolioRelationView {
                direction: relation.direction.to_string(),
                other_project: relation.other_project,
                relation_type: relation.relation_type.label().to_string(),
                note: relation.note.unwrap_or_default(),
            })
            .collect(),
        evidence: view
            .evidence
            .into_iter()
            .map(|snapshot| EvidenceRowView {
                source_system: snapshot.source_system,
                source_revision: snapshot.source_revision,
                observed_at: snapshot.observed_at,
                status: snapshot.effective_status,
                stale_after: snapshot.stale_after.unwrap_or_default(),
            })
            .collect(),
        reviews: view
            .reviews
            .into_iter()
            .map(|review| ReviewRowView {
                reviewed_at: review.reviewed_at,
                confidence: review.confidence.label().to_string(),
                note: review.note.unwrap_or_default(),
            })
            .collect(),
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

/// The user-owned edits one `POST /ui/projects/{id}/portfolio`
/// can carry. Every field is optional and empty fields are
/// refused rather than silently clearing stored values: the form
/// posts only the fields the operator actually filled in.
#[derive(Debug, Clone, Default)]
pub struct PortfolioFormInput {
    pub lifecycle: Option<String>,
    pub confidence: Option<String>,
    pub next_action: Option<String>,
    pub blocker: Option<String>,
    pub tag: Option<String>,
    pub remove_tag: Option<String>,
}

/// One applied portfolio edit, rendered back as the confirmation
/// list on the detail page.
#[derive(Debug, Clone)]
pub struct PortfolioEdit {
    pub action: String,
    pub detail: String,
}

/// Apply the operator's portfolio edits through the same Core
/// contracts the CLI and JSON API use. Returns the applied edits
/// in request order so the caller can render what changed.
///
/// Only user-owned facts are writable here. Imported evidence is
/// append-only and arrives through `forge portfolio evidence
/// import` or `POST /v1/projects/{id}/portfolio/evidence`; no
/// browser form can rewrite a source-owned snapshot, and no form
/// here writes repository or provider files.
pub fn apply_portfolio_form(
    db_path: &Path,
    project_id: &str,
    input: &PortfolioFormInput,
) -> Result<Vec<PortfolioEdit>, ForgeError> {
    if input.tag.is_none()
        && input.remove_tag.is_none()
        && input.lifecycle.is_none()
        && input.confidence.is_none()
        && input.next_action.is_none()
        && input.blocker.is_none()
    {
        return Err(ForgeError::PortfolioInvalid {
            reason: "portfolio form carried no editable field".to_string(),
        });
    }
    let registry = Registry::open(db_path)?;
    let mut edits = Vec::new();

    if let Some(name) = &input.tag {
        let tag = registry.portfolio_add_tag(project_id, name, None)?;
        edits.push(PortfolioEdit {
            action: "tag".to_string(),
            detail: format!("tagged with {}", tag.name),
        });
    }
    if let Some(name) = &input.remove_tag {
        let removed = registry.portfolio_remove_tag(project_id, name)?;
        edits.push(PortfolioEdit {
            action: "untag".to_string(),
            detail: if removed {
                format!("removed tag {name}")
            } else {
                format!("tag {name} was not attached; nothing changed")
            },
        });
    }

    // A review is the only write that carries confidence, so a
    // confidence change without the other fields still records one.
    let touches_classification = input.lifecycle.is_some()
        || input.confidence.is_some()
        || input.next_action.is_some()
        || input.blocker.is_some();
    if touches_classification {
        let lifecycle = input
            .lifecycle
            .as_deref()
            .map(Lifecycle::parse)
            .transpose()
            .map_err(portfolio_invalid)?;
        let confidence = input
            .confidence
            .as_deref()
            .map(Confidence::parse)
            .transpose()
            .map_err(portfolio_invalid)?;
        let existing = registry.portfolio_record(project_id)?;
        let write = crate::registry::PortfolioWrite {
            lifecycle: lifecycle.or(existing.lifecycle),
            confidence: confidence.or(existing.confidence),
            next_action: merge_note(&existing.next_action, input.next_action.as_deref())?,
            blocker: merge_note(&existing.blocker, input.blocker.as_deref())?,
        };
        let profile = registry.portfolio_write(project_id, &write)?;
        let mut detail_parts: Vec<String> = Vec::new();
        if let Some(lifecycle) = input.lifecycle.as_deref() {
            detail_parts.push(format!("lifecycle {lifecycle}"));
        }
        if let Some(confidence) = input.confidence.as_deref() {
            detail_parts.push(format!("confidence {confidence}"));
        }
        if input.next_action.is_some() {
            detail_parts.push("next action set".to_string());
        }
        if input.blocker.is_some() {
            detail_parts.push("blocker set".to_string());
        }
        let review = match confidence {
            Some(value) => Some(registry.portfolio_record_review(project_id, value, None)?),
            None => None,
        };
        if let Some(review) = review {
            detail_parts.push(format!("reviewed at {}", review.reviewed_at));
        }
        edits.push(PortfolioEdit {
            action: "review".to_string(),
            detail: if detail_parts.is_empty() {
                format!(
                    "reviewed; reviewed_at {}",
                    profile.reviewed_at.unwrap_or_default()
                )
            } else {
                detail_parts.join(", ")
            },
        });
    }

    if edits.is_empty() {
        return Err(ForgeError::PortfolioInvalid {
            reason: "portfolio form carried no applicable editable field".to_string(),
        });
    }
    // The journal keeps the operator's edit auditable without
    // claiming an external observation happened.
    let summary = edits
        .iter()
        .map(|edit| format!("{}: {}", edit.action, edit.detail))
        .collect::<Vec<String>>()
        .join("; ");
    let _ = registry.record_operation("portfolio.ui", project_id, "done", &summary);
    Ok(edits)
}

fn portfolio_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInvalid { reason }
}

/// Merge one optional free-text field. An empty submitted field
/// means "leave the stored value alone"; clearing a field is a CLI
/// operation, never a side effect of an empty form box.
fn merge_note(
    existing: &Option<String>,
    submitted: Option<&str>,
) -> Result<Option<String>, ForgeError> {
    let Some(raw) = submitted else {
        return Ok(existing.clone());
    };
    let value = crate::portfolio::validate_note("field", raw, crate::portfolio::MAX_NOTE_CHARS)
        .map_err(portfolio_invalid)?;
    Ok(value.or_else(|| existing.clone()))
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
