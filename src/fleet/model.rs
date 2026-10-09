//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

/// One excluded malformed entry: named with the reason, never silently
/// dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetMalformedEntry {
    pub name: String,
    pub reason: String,
}
/// Management state of a fleet entry: only the local registry decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FleetState {
    /// The same project id exists in the local registry.
    Managed,
    /// The local registry has no record of this id; mirroring never
    /// registers, imports or mutates it.
    Unmanaged,
}
impl FleetState {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            FleetState::Managed => "managed",
            FleetState::Unmanaged => "unmanaged",
        }
    }
}
/// One projected fleet entry. `profile` and `lifecycle` are surfaced
/// verbatim from the Workspace Governance vocabulary (never coerced to
/// Forge profiles) after credential redaction and length bounding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetEntry {
    pub id: String,
    pub path: String,
    pub profile: String,
    pub lifecycle: String,
    #[serde(default)]
    pub adoption: Option<String>,
    pub forge_yaml_present: bool,
    pub locally_registered: bool,
    pub state: FleetState,
}
/// The timestamped fleet report. Re-read on demand; never persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetReport {
    pub contract: String,
    /// Absolute path of the registry document, or `None` when unconfigured.
    pub source: Option<String>,
    pub observed_at: String,
    pub freshness: FleetFreshness,
    pub max_age_seconds: i64,
    /// Age of the registry document in whole seconds at observation time.
    pub age_seconds: Option<i64>,
    pub entries: Vec<FleetEntry>,
    pub malformed: Vec<FleetMalformedEntry>,
}
impl FleetReport {
    /// Total declared entries (valid plus malformed) so a consumer can
    /// tell projection from drop.
    pub fn declared_count(&self) -> usize {
        self.entries.len() + self.malformed.len()
    }
}
/// Registry freshness classification from the file mtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FleetFreshness {
    /// The registry document is newer than the configured max age.
    Fresh,
    /// The registry document is older than the configured max age.
    Stale,
    /// No registry path is configured; nothing was contacted.
    Unconfigured,
}
impl FleetFreshness {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            FleetFreshness::Fresh => "fresh",
            FleetFreshness::Stale => "stale",
            FleetFreshness::Unconfigured => "unconfigured",
        }
    }
}
