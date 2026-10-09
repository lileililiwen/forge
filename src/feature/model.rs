//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// One deterministic plan step: install an exact version.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PlanStep {
    pub feature: String,
    pub version: String,
    pub action: String,
}
pub(super) struct Rollback {
    pub(super) manifest_path: PathBuf,
    pub(super) manifest_before: Vec<u8>,
    pub(super) receipts_written: Vec<PathBuf>,
    pub(super) receipts_deleted: Vec<(PathBuf, String)>,
}
impl Rollback {
    pub(super) fn restore(&self) {
        let _ = fs::write(&self.manifest_path, &self.manifest_before);
        for path in &self.receipts_written {
            let _ = fs::remove_file(path);
        }
        for (path, contents) in &self.receipts_deleted {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, contents);
        }
    }
}
/// Versioned descriptor for one catalog feature.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub id: String,
    pub version: String,
    pub compatible_profiles: Vec<String>,
    pub depends: Vec<String>,
    pub conflicts: Vec<String>,
    pub install_strategy: String,
    pub upgrade_strategy: String,
    pub validation: Vec<String>,
    pub documentation: String,
    pub tests: String,
}
/// Reviewable deterministic plan: exact versions in dependency order plus
/// the declared validators the operation will report.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FeaturePlan {
    pub profile: String,
    pub requested: Vec<String>,
    pub steps: Vec<PlanStep>,
    pub validators: Vec<String>,
}
/// Outcome of a lifecycle operation. `changed == false` is a no-op: the
/// manifest, receipts and registry journal are untouched.
#[derive(Debug, Clone, Serialize)]
pub struct LifecycleOutcome {
    pub project_id: String,
    pub profile: String,
    pub operation: String,
    pub plan: FeaturePlan,
    pub changed: bool,
    pub note: String,
    pub files_changed: Vec<String>,
    pub features: BTreeMap<String, String>,
}
