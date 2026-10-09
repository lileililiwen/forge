//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

/// Per-outcome totals of a [`sync_workspace`] run.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceSyncSummary {
    pub ok: usize,
    pub already: usize,
    pub skipped: usize,
    pub failed: usize,
}
/// Read-only import proposal for one directory.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportProposal {
    pub path: String,
    pub language: Detection,
    pub framework: Detection,
    pub package_manager: Detection,
    pub database: Detection,
    pub docker: Detection,
    pub ci: Detection,
    pub auth: Detection,
    pub features: Detection,
    pub driftwatch: Detection,
    pub git_remote: Detection,
    pub deployment: Detection,
    /// Presence of a Workspace Governance `.project.json`. Informational
    /// only: import never creates, rewrites or removes the file.
    pub workspace_metadata: Detection,
    pub suggested_profile: Option<String>,
    pub suggested_maturity: Option<String>,
    pub confidence: String,
    pub alternatives: Vec<String>,
    pub explicit_profile: bool,
    pub manifest_exists: bool,
}
/// The complete per-directory report of one [`sync_workspace`] run.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSyncReport {
    pub root: String,
    pub entries: Vec<WorkspaceSyncEntry>,
    pub summary: WorkspaceSyncSummary,
}
impl WorkspaceSyncReport {
    /// The run fails (non-zero CLI exit) iff at least one directory failed.
    /// Skips never fail the run.
    pub fn failed(&self) -> bool {
        self.summary.failed > 0
    }
}
/// One inventoried area with its evidence files or probe notes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Detection {
    pub status: FieldStatus,
    pub value: Option<String>,
    pub evidence: Vec<String>,
}
impl Detection {
    pub(super) fn detected(value: impl Into<String>, evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Detected,
            value: Some(value.into()),
            evidence,
        }
    }
    pub(super) fn missing(evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Missing,
            value: None,
            evidence,
        }
    }
    pub(super) fn unknown(evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Unknown,
            value: None,
            evidence,
        }
    }
}
/// One directory outcome of [`sync_workspace`]. `outcome` is closed
/// vocabulary: `ok` (registered or adopted now), `already` (registered at
/// the same path before this run, nothing rewritten), `skipped` (never a
/// failure: hidden, non-directory, symlink, ambiguous, undecidable or
/// unreadable entry) or `failed` (typed `reason`, siblings still
/// processed). `id`/`profile` ride `ok` and `already`; `reason` rides
/// `skipped` and `failed`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceSyncEntry {
    pub directory: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
pub(super) struct Candidate {
    pub(super) profile: &'static str,
    pub(super) score: i32,
}
/// How an import inventory area was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldStatus {
    /// Direct evidence was found.
    Detected,
    /// The area was probed and is absent.
    Missing,
    /// No evidence was available to decide.
    Unknown,
}
