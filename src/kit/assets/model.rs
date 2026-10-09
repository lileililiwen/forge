//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

/// Per-file verification outcome for the vendored tree.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetState {
    /// Bytes match the digest recorded in `kits/manifest.json`.
    Match,
    /// Bytes differ from the recorded digest.
    Drift,
    /// The file named by the descriptor is absent from the vendored tree.
    Missing,
}
/// A read-only upgrade plan. Never writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitDiffReport {
    pub path: String,
    pub project: String,
    pub profile: String,
    pub from: String,
    pub against: String,
    pub entries: Vec<KitDiffEntry>,
    pub conflicts: Vec<String>,
}
/// One file's upgrade plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitDiffEntry {
    pub path: String,
    pub change: KitDiffChange,
    /// The rendered before/after text, so the plan is reviewable without a
    /// separate diff tool.
    pub before: String,
    pub after: String,
}
/// A project with no kit-owned subtree, or a kit the registry cannot resolve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitUnavailable {
    pub path: String,
    /// Why no upgrade path exists. Never reported as current.
    pub reason: String,
}
/// Outcome of checking a committed feed against a declared kit version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedVerificationReport {
    pub kit: String,
    /// The version the project declares in `forge.yaml` `kit.version`.
    pub declared_version: String,
    /// Where the declared version came from: the compiled-in descriptor or a
    /// project's own manifest. Recorded so a failure names the real source.
    pub declared_by: String,
    pub feed_path: String,
    pub packages: Vec<FeedVerificationEntry>,
    /// Whether every committed package's bytes were checked against a recorded
    /// digest. `false` means the feed was version-checked only, and the report
    /// must not be read as byte-verified.
    pub digests_verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedVerificationEntry {
    pub package: String,
    pub version: String,
    pub file: String,
    pub role: String,
    /// `present` or `missing`.
    pub state: String,
}
/// What a kit upgrade would change for one owned path. The vocabulary is
/// deliberately identical to [`crate::standard::DiffChange`]: one ownership
/// discipline, not two.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KitDiffChange {
    /// Target file absent on disk: the upgrade would create it.
    Added,
    /// Present, receipt-owned and unedited: the upgrade would update it.
    Updated,
    /// Present and byte-identical to the target: nothing to do.
    Unchanged,
    /// Present, receipt-owned but edited by the operator: conflict.
    Modified,
    /// Present, not receipt-owned, different bytes: never clobbered.
    Foreign,
    /// Receipt-owned but absent from the target kit: preserved, not removed.
    Orphaned,
}
/// The result of an applied kit upgrade.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitUpgradeReport {
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
/// Ownership receipt for the kit-owned subtree. Mirrors
/// [`crate::standard::Receipt`]: one digest per owned file, so a later user
/// edit is visible as a conflict instead of an overwrite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitReceipt {
    pub schema: u8,
    pub generator: String,
    pub project: String,
    pub profile: String,
    pub kit: String,
    pub version: Option<String>,
    pub files: Vec<KitReceiptFile>,
}
/// One owned kit file plus its digest, as recorded in the project receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitReceiptFile {
    pub path: String,
    pub digest: String,
}
/// One vendored file's provenance, as recorded in `kits/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ManifestFile {
    pub(super) path: String,
    pub(super) sha256: String,
    #[serde(default)]
    #[serde(rename = "source")]
    pub(super) source_name: Option<String>,
    #[serde(default)]
    pub(super) revision: Option<String>,
}
/// The vendored-asset manifest. Same discipline as
/// `contracts/manifest.json`: schema version, owning source, the source
/// revision and one digest per file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct KitManifest {
    pub(super) schema_version: u64,
    pub(super) source: String,
    pub(super) revision: String,
    #[serde(default)]
    pub(super) synced_at: String,
    pub(super) files: Vec<ManifestFile>,
}
/// One file's verification result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitAssetReport {
    pub path: String,
    pub state: AssetState,
    pub expected: String,
    pub actual: String,
}
