//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{CapturedChangelog, CapturedCheck, ReleaseConfig, ReleaseIdentity};
use serde::{Deserialize, Serialize};

/// Plan-only release report. The transport renders this
/// before any side effect runs; the contract refuses to apply
/// a plan that is not `ready`.
#[derive(Debug, Clone, Serialize)]
pub struct PlanReport {
    pub contract: String,
    pub project_id: String,
    pub identity: ReleaseIdentity,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
    pub checks: Vec<CapturedCheck>,
    pub stages: Vec<String>,
    pub note: String,
    pub ready: bool,
}
impl PlanReport {
    pub fn healthy(&self) -> bool {
        self.ready
    }
}
/// List every persisted release under the project. The
/// result is the inventory the CLI renders for
/// `forge release list`.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseListEntry {
    pub project_id: String,
    pub release_id: String,
    pub version: String,
    pub source_revision: String,
    pub stage_count: usize,
    pub last_run_at: String,
}
/// One configuration captured at prepare time. The apply step
/// reads this back from the persisted state to keep the
/// captured checks and the captured changelog bound to the
/// release identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedConfig {
    pub config: ReleaseConfig,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
}
