//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Read-only `check` outcome for a project directory.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SnapshotReport {
    pub state: SnapshotState,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_digest_matches: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<String>,
}
/// One asset template owned by a pack. `path` and `template` may contain
/// `{id}`, `{pack}`, `{version}`, `{profile}`, `{build_command}` and
/// `{test_command}` placeholders substituted at render time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackAsset {
    pub path: String,
    pub template: String,
}
/// What a target snapshot would change for one path.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiffChange {
    /// Target file absent on disk: upgrade would create it.
    Added,
    /// Present, receipt-owned and unedited: upgrade would update it.
    Updated,
    /// Present and byte-identical to the target: nothing to do.
    Unchanged,
    /// Present, receipt-owned but edited by the user: conflict.
    Modified,
    /// Present, not receipt-owned, different bytes: conflict (never clobber).
    Foreign,
    /// Receipt-owned but absent from the target pack: preserved, not removed.
    Orphaned,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiffEntry {
    pub path: String,
    pub change: DiffChange,
}
/// Evidence behind a pack's support claim. A pack without verified render
/// fixtures is never selectable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackEvidence {
    /// Render/contract fixtures exist for this pack version.
    Verified,
    /// No verified fixture yet.
    Unverified,
}
/// Ownership receipt. Written into the project; the recorded digests make a
/// later user edit visible as a conflict instead of an overwrite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub schema: u8,
    pub generator: String,
    pub project: String,
    pub profile: String,
    pub pack: String,
    pub version: String,
    pub asset_digest: String,
    pub generated_at: String,
    pub files: Vec<ReceiptFile>,
}
/// Snapshot lifecycle state as observed on disk.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotState {
    /// No Forge-owned receipt: nothing to manage.
    Absent,
    /// Receipt matches every owned file byte-for-byte.
    Rendered,
    /// An owned file is missing or differs from its recorded digest.
    Modified,
    /// The receipt names a pack version this build does not know.
    Unknown,
}
/// A fully rendered snapshot: owned content files plus the receipt that
/// records them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSnapshot {
    pub pack: String,
    pub version: String,
    pub profile: String,
    pub asset_digest: String,
    /// Content files (every owned path except the receipt), sorted by path.
    pub files: Vec<(String, String)>,
    pub receipt: Receipt,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileReport {
    pub path: String,
    pub state: FileState,
}
/// Result of an applied upgrade.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpgradeReport {
    pub path: String,
    pub project: String,
    pub profile: String,
    pub to: String,
    pub written: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forced: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub orphaned: Vec<String>,
    pub generated_at: String,
}
/// Versioned standard-pack descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackDescriptor {
    pub id: String,
    pub version: String,
    pub support_state: PackSupportState,
    pub evidence: PackEvidence,
    /// Profiles this pack may be materialized for.
    pub compatible_profiles: Vec<String>,
    /// Optional external template origin. It is never fetched: when the
    /// origin is unavailable Forge uses its bundled local fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_source: Option<String>,
    /// SHA-256 over the pack's canonical asset listing. Stable for a given
    /// pack version and independent of any project input.
    pub asset_digest: String,
    /// Backing assets. Skipped from transport output; consumers use the
    /// rendered snapshot, not the templates.
    #[serde(skip)]
    pub files: Vec<PackAsset>,
}
impl PackDescriptor {
    /// True when this pack version may be selected for new generation.
    pub fn is_selectable(&self) -> bool {
        self.support_state == PackSupportState::Supported && self.evidence == PackEvidence::Verified
    }
    /// True when the pack may be materialized for `profile`.
    pub fn supports(&self, profile: &str) -> bool {
        self.compatible_profiles.iter().any(|p| p == profile)
    }
}
/// Catalog support state for one pack version.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackSupportState {
    /// Reserved on the roadmap; not selectable.
    Proposed,
    /// Selectable: has a tested render fixture and a compatible profile.
    Supported,
    /// Still inspectable and usable by a pinned project, but new
    /// generation refuses it.
    Deprecated,
}
/// Read-only diff against a target pack version.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiffReport {
    pub path: String,
    pub project: String,
    pub profile: String,
    pub from: String,
    pub against: String,
    pub entries: Vec<DiffEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}
/// Per-file observation state in a `check`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileState {
    Present,
    Modified,
    Missing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackAssetOrigin {
    /// Bundled local assets (the always-available fallback).
    LocalFallback,
    /// A verified external template directory.
    External(PathBuf),
}
/// One rendered owned file plus its digest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptFile {
    pub path: String,
    pub digest: String,
}
