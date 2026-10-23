//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

/// Outcome of a single-project upgrade application.
#[derive(Debug, Clone, Serialize)]
pub struct UpgradeOutcome {
    pub project_id: String,
    pub profile: String,
    pub operation: String,
    pub plan: UpgradePlan,
    pub changed: bool,
    pub note: String,
    pub files_changed: Vec<String>,
    pub features: std::collections::BTreeMap<String, String>,
    pub validation: Vec<String>,
    pub recovery: Vec<String>,
}
/// One deterministic upgrade step for a single feature.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpgradeStep {
    pub feature: String,
    pub old_version: Option<String>,
    pub new_version: String,
    pub action: String,
    /// Step kinds in preferred order: package, configuration, codemod,
    /// schema (only for data-migration features), replacement (none in v0.2).
    pub kinds: Vec<String>,
    pub assets: Vec<String>,
    pub validators: Vec<String>,
    pub migration_strategy: String,
    pub recovery: String,
    pub reversible: bool,
}
/// Structured semantic-conflict handoff for later spec-generation
/// integration. No AI editing is performed; the conflict names the owned
/// file, the blocking reason and the suggested `forge spec generate`
/// follow-up.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SemanticConflict {
    pub project_id: String,
    pub feature: String,
    pub owned_file: String,
    pub reason: String,
    pub suggested_spec: String,
}
/// Pinned read-only upgrade plan for one project.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpgradePlan {
    pub contract: String,
    pub project_id: String,
    pub profile: String,
    pub requested: Vec<String>,
    pub steps: Vec<UpgradeStep>,
    pub validators: Vec<String>,
}
/// One per-project fleet entry with a distinct status.
#[derive(Debug, Clone, Serialize)]
pub struct FleetEntry {
    pub project_id: String,
    pub status: String,
    pub changed: bool,
    pub note: String,
    pub features: std::collections::BTreeMap<String, String>,
    pub validation: Vec<String>,
    pub recovery: Vec<String>,
    pub conflict: Option<SemanticConflict>,
}
/// The fleet's verdict: what the run established about the fleet.
///
/// A verdict is never a second opinion: it is derived once from the
/// per-project outcomes and read by every surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetVerdict {
    /// Every selected project was judged, and none failed or blocked.
    Healthy,
    /// At least one selected project could not be judged.
    Unavailable,
    /// At least one selected project failed or blocked.
    NotHealthy,
}
impl FleetVerdict {
    /// The verdict's own word. The human renderer and the JSON document
    /// both state this exact string, so the two surfaces never disagree.
    pub fn as_str(&self) -> &'static str {
        match self {
            FleetVerdict::Healthy => "healthy",
            FleetVerdict::Unavailable => "unavailable",
            FleetVerdict::NotHealthy => "not healthy",
        }
    }
    /// Derive the verdict from the per-project outcomes of one run.
    ///
    /// A fleet is healthy only when every selected project was judged and
    /// none failed or blocked. A project that could not be judged makes the
    /// verdict `unavailable`, never `healthy`. A definite failure outranks
    /// an unknown: a fleet with one failed project and one unavailable
    /// project is `not healthy`.
    pub fn from_entries(entries: &[FleetEntry]) -> FleetVerdict {
        let mut unjudged = false;
        for entry in entries {
            match entry.status.as_str() {
                "failure" | "blocked" => return FleetVerdict::NotHealthy,
                "unavailable" => unjudged = true,
                "success" | "skipped" => {}
                // An unrecognised result is never read as judged.
                _ => unjudged = true,
            }
        }
        if unjudged {
            FleetVerdict::Unavailable
        } else {
            FleetVerdict::Healthy
        }
    }
}
impl Serialize for FleetVerdict {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
/// Aggregated fleet report over an explicit captured selection.
#[derive(Debug, Clone, Serialize)]
pub struct FleetReport {
    pub contract: String,
    pub selection: Vec<String>,
    pub requested: Vec<String>,
    pub dry_run: bool,
    /// The one verdict every surface reads.
    pub verdict: FleetVerdict,
    pub entries: Vec<FleetEntry>,
    pub succeeded: usize,
    pub failed: usize,
    pub blocked: usize,
    /// Projects the fleet judged and chose not to change (already satisfied,
    /// or a dry-run plan).
    pub skipped: usize,
    /// Projects the fleet could not judge (their record was unreadable, or
    /// their path was unavailable).
    pub unavailable: usize,
}
impl FleetReport {
    /// A fleet is healthy only when its verdict says so: every selected
    /// project was judged and none failed or blocked.
    pub fn healthy(&self) -> bool {
        self.verdict == FleetVerdict::Healthy
    }
}
