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
/// Aggregated fleet report over an explicit captured selection.
#[derive(Debug, Clone, Serialize)]
pub struct FleetReport {
    pub contract: String,
    pub selection: Vec<String>,
    pub requested: Vec<String>,
    pub dry_run: bool,
    pub entries: Vec<FleetEntry>,
    pub succeeded: usize,
    pub failed: usize,
    pub blocked: usize,
    pub skipped: usize,
}
impl FleetReport {
    /// A fleet is healthy only when no project failed or blocked.
    /// Skipped (already satisfied, unavailable, dry-run) is healthy.
    pub fn healthy(&self) -> bool {
        self.failed == 0 && self.blocked == 0
    }
}
