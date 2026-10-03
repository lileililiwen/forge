//! Digest-pinned access to the vendored `kits/` tree.
//!
//! `kits/` is deliberately the same shape as `contracts/` in
//! `platform-contract-consumption`: a checked-in directory beside the
//! crate, a `manifest.json` recording the source revision and the sha256 of
//! every vendored file, and an offline verification pass. Nothing is fetched
//! and no sibling checkout is read at render time — the bytes Forge stages
//! are the bytes it verified.
//!
//! The vendored assets are the **generated** token pair, not the DTCG source:
//! the DTCG file stays upstream-owned, and the project compiles against the
//! generated pair. A tampered byte fails generation with `kit-digest-mismatch`
//! rather than producing a project that silently differs from its descriptor.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

/// Owned subtree in a generated project holding the vendored token source.
pub const PLATFORM_TOKENS_DIR: &str = ".platform/tokens";

/// Ownership receipt for the kit-owned subtree. A separate receipt rather
/// than a `.standard/` pack, because reusing that machinery requires a
/// standard-pack descriptor owned by the sibling library, which is outside
/// this repository's boundary. The shapes and the refusal vocabulary are
/// deliberately identical to the standard snapshot receipt.
pub const PLATFORM_RECEIPT_PATH: &str = ".platform/receipt.json";

/// Version of the kit receipt document.
pub const PLATFORM_RECEIPT_SCHEMA: u8 = 1;

/// Directory holding the checked-in vendored assets.
pub fn kits_dir() -> PathBuf {
    if let Some(dir) =
        std::env::var_os("FORGE_KITS_DIR").filter(|v| !v.to_string_lossy().trim().is_empty())
    {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kits")
}

/// One vendored file's provenance, as recorded in `kits/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestFile {
    path: String,
    sha256: String,
    #[serde(default)]
    #[serde(rename = "source")]
    source_name: Option<String>,
    #[serde(default)]
    revision: Option<String>,
}

/// The vendored-asset manifest. Same discipline as
/// `contracts/manifest.json`: schema version, owning source, the source
/// revision and one digest per file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KitManifest {
    schema_version: u64,
    source: String,
    revision: String,
    #[serde(default)]
    synced_at: String,
    files: Vec<ManifestFile>,
}

fn load_manifest() -> Result<KitManifest, ForgeError> {
    let path = kits_dir().join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kits/manifest.json missing or unreadable at {}: {err}; \
             nothing was staged and no project was registered",
            path.display()
        ),
    })?;
    serde_json::from_str(&text).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kits/manifest.json is not a valid kit manifest: {err}; \
             nothing was staged and no project was registered"
        ),
    })
}

fn digest_of(bytes: &[u8]) -> String {
    crate::standard::sha256_hex(bytes)
}

/// The digest-pinned vendored assets of the `platform-ui-web` kit, with the
/// path each one takes inside a generated project.
///
/// Compiled in, so the descriptor is a property of the kit version alone and
/// identical inputs always render identical bytes.
pub fn token_assets() -> Vec<crate::kit::registry::AssetRef> {
    vec![
        crate::kit::registry::AssetRef {
            path: "tokens/tokens.css".to_string(),
            sha256: "720d0bc7afa7a8e3d1936e6109fd1ec42b2c5dc2268bb863dc68df6c8382b191".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/tokens.css"),
            binary: false,
        },
        crate::kit::registry::AssetRef {
            path: "tokens/tokens.ts".to_string(),
            sha256: "98dfd642d02a194485cc58b4948a4110319d706cf6bd7b35be4ed4472a614f38".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/tokens.ts"),
            binary: false,
        },
        crate::kit::registry::AssetRef {
            path: "scripts/verify-tokens.mjs".to_string(),
            sha256: "29c7f0bfb531141d8747ab4c58aacf7f90e776a1d0af1a33a2e397f9fd37ed82".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/verify-tokens.mjs"),
            binary: false,
        },
    ]
}

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

/// One file's verification result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitAssetReport {
    pub path: String,
    pub state: AssetState,
    pub expected: String,
    pub actual: String,
}

/// Verify every vendored asset against the digest recorded in
/// `kits/manifest.json`.
///
/// The first drifted file fails with `kit-digest-mismatch` naming the file,
/// the expected digest and the actual digest, so a tampered asset can never
/// reach a generated project.
pub fn verify_kit_digests() -> Result<Vec<KitAssetReport>, ForgeError> {
    let manifest = load_manifest()?;
    let base = kits_dir();
    let mut reports = Vec::with_capacity(manifest.files.len());
    for entry in &manifest.files {
        let file = base.join(&entry.path);
        let actual = match fs::read(&file) {
            Ok(bytes) => digest_of(&bytes),
            Err(err) => {
                return Err(ForgeError::KitDigestMismatch {
                    reason: format!(
                        "vendored kit asset '{}' is unreadable at {}: {err} (expected {}); \
                         nothing was staged and no project was registered",
                        entry.path,
                        file.display(),
                        entry.sha256
                    ),
                })
            }
        };
        if actual != entry.sha256 {
            return Err(ForgeError::KitDigestMismatch {
                reason: format!(
                    "vendored kit asset '{}' has drifted: expected {} got {}; \
                     nothing was staged and no project was registered",
                    entry.path, entry.sha256, actual
                ),
            });
        }
        reports.push(KitAssetReport {
            path: entry.path.clone(),
            state: AssetState::Match,
            expected: entry.sha256.clone(),
            actual,
        });
    }
    Ok(reports)
}

/// Read every asset a descriptor declares, verifying its digest first.
///
/// Returns the staged `(path, contents)` pairs in stable path order. This is
/// the only path by which kit bytes reach a generated project, so a drifted
/// asset is caught here — before anything is staged — rather than after.
pub fn read_verified_assets(
    descriptor: &crate::kit::registry::KitDescriptor,
) -> Result<Vec<(String, String)>, ForgeError> {
    // Only text assets belong to the `.platform/` owned subtree and its
    // receipt. Feed `.nupkg` bytes are binary and live in the feed directory
    // the package-manager configuration names, so they are read by
    // `read_verified_feed_assets` instead.
    let text_assets: Vec<&crate::kit::registry::AssetRef> =
        descriptor.assets.iter().filter(|a| !a.binary).collect();
    if text_assets.is_empty() {
        return Ok(Vec::new());
    }
    // Verify the whole tree first: a drifted sibling is a broken kit, and a
    // partially vendored token source is worse than none.
    verify_kit_digests()?;
    let base = kits_dir();
    let mut out = Vec::with_capacity(text_assets.len());
    for asset in text_assets {
        let bytes =
            fs::read(base.join(&asset.path)).map_err(|err| ForgeError::KitDigestMismatch {
                reason: format!(
                    "vendored kit asset '{}' declared by kit '{}' is unreadable at {}: {err}; \
                 nothing was staged and no project was registered",
                    asset.path,
                    descriptor.reference.id,
                    base.join(&asset.path).display()
                ),
            })?;
        let actual = digest_of(&bytes);
        if actual != asset.sha256 {
            return Err(ForgeError::KitDigestMismatch {
                reason: format!(
                    "vendored kit asset '{}' has drifted from the descriptor for kit '{}': \
                     expected {} got {}; nothing was staged and no project was registered",
                    asset.path, descriptor.reference.id, asset.sha256, actual
                ),
            });
        }
        let text = String::from_utf8(bytes).map_err(|err| ForgeError::KitDigestMismatch {
            reason: format!(
                "vendored kit asset '{}' is not valid UTF-8: {err}; \
                 nothing was staged and no project was registered",
                asset.path
            ),
        })?;
        out.push((asset.target.clone(), text));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// The recorded digest for one vendored path, or `None` when the manifest does
/// not mention it.
pub fn manifest_digest(path: &str) -> Result<Option<String>, ForgeError> {
    Ok(load_manifest()?
        .files
        .iter()
        .find(|f| f.path == path)
        .map(|f| f.sha256.clone()))
}

/// Read every committed feed package a descriptor declares, verifying each
/// against the digest recorded in `kits/manifest.json`.
///
/// This is the only path by which feed bytes reach a generated project, so a
/// tampered `.nupkg` is caught here — before anything is staged — rather than
/// after. Returns `(target path inside the project, bytes)` in stable
/// package-name order.
///
/// The digest comes from the manifest rather than from the compiled-in
/// descriptor on purpose: `dotnet pack` is not byte-reproducible, so a digest
/// frozen in the binary would turn every legitimate repack into a failure and
/// teach operators to re-record it without reading. The *version* is the
/// compiled-in fact, and that is what the drift check holds.
pub fn read_verified_feed_assets(
    descriptor: &crate::kit::registry::KitDescriptor,
) -> Result<Vec<(String, Vec<u8>)>, ForgeError> {
    if descriptor.feed_packages.is_empty() {
        return Ok(Vec::new());
    }
    verify_kit_digests()?;
    let base = kits_dir();
    let mut out = Vec::with_capacity(descriptor.feed_packages.len());
    for entry in &descriptor.feed_packages {
        let relative = entry.kits_path();
        let file = base.join(&relative);
        let bytes = fs::read(&file).map_err(|err| ForgeError::KitFeedIncomplete {
            reason: format!(
                "committed feed package '{}' is unreadable at {}: {err}; a fresh clone could not \
                 restore it; nothing was staged and no project was registered",
                entry.file,
                file.display()
            ),
        })?;
        let expected =
            manifest_digest(&relative)?.ok_or_else(|| ForgeError::KitDigestMismatch {
                reason: format!(
                    "committed feed package '{relative}' has no digest in kits/manifest.json; \
                 run `forge kit pack` to record it; nothing was staged and no project was \
                 registered"
                ),
            })?;
        let actual = digest_of(&bytes);
        if actual != expected {
            return Err(ForgeError::KitDigestMismatch {
                reason: format!(
                    "committed feed package '{relative}' has drifted: expected {expected} got \
                     {actual}; nothing was staged and no project was registered"
                ),
            });
        }
        out.push((entry.target(), bytes));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// One owned kit file plus its digest, as recorded in the project receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitReceiptFile {
    pub path: String,
    pub digest: String,
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

/// Build the receipt for the files a kit owns. Pure: identical inputs give
/// identical bytes, including the receipt itself.
pub fn render_receipt(
    id: &str,
    profile: &str,
    descriptor: &crate::kit::registry::KitDescriptor,
    owned: &[(String, String)],
) -> KitReceipt {
    let mut files: Vec<KitReceiptFile> = owned
        .iter()
        .map(|(path, content)| KitReceiptFile {
            path: path.clone(),
            digest: digest_of(content.as_bytes()),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    KitReceipt {
        schema: PLATFORM_RECEIPT_SCHEMA,
        generator: format!("forge@{}", env!("CARGO_PKG_VERSION")),
        project: id.to_string(),
        profile: profile.to_string(),
        kit: descriptor.reference.id.clone(),
        version: descriptor.reference.version.clone(),
        files,
    }
}

/// Serialize a kit receipt as its canonical document.
pub fn receipt_text(receipt: &KitReceipt) -> Result<String, ForgeError> {
    let text =
        serde_json::to_string_pretty(receipt).map_err(|err| ForgeError::KitDigestMismatch {
            reason: format!("kit receipt is not serializable: {err}"),
        })?;
    Ok(format!("{text}\n"))
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

/// A project with no kit-owned subtree, or a kit the registry cannot resolve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitUnavailable {
    pub path: String,
    /// Why no upgrade path exists. Never reported as current.
    pub reason: String,
}

fn read_receipt(dir: &Path) -> Result<Option<KitReceipt>, ForgeError> {
    let path = dir.join(PLATFORM_RECEIPT_PATH);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ForgeError::KitDigestMismatch {
                reason: format!("cannot read kit receipt '{}': {err}", path.display()),
            })
        }
    };
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kit receipt '{}' is not a valid receipt: {err}",
            path.display()
        ),
    })
}

/// Plan the difference between a project's kit-owned subtree and the target
/// kit version. Read-only; never writes.
///
/// Returns `Err` only for a genuinely broken tree. A project that cannot be
/// upgraded at all — a declared zero, or a pinned kit this build no longer
/// knows — returns an explicit "unavailable, with a reason" state rather than
/// being reported as current.
pub fn diff_kit_snapshot(
    dir: &Path,
    against: Option<&str>,
) -> Result<Result<KitDiffReport, KitUnavailable>, ForgeError> {
    let Some(receipt) = read_receipt(dir)? else {
        return Ok(Err(KitUnavailable {
            path: dir.display().to_string(),
            reason: format!(
                "no Forge-owned kit subtree ({PLATFORM_RECEIPT_PATH} absent); this project pinned \
                 no shared-layer kit, so there is no kit upgrade to plan"
            ),
        }));
    };
    let Some(against) = against else {
        return Ok(Err(KitUnavailable {
            path: dir.display().to_string(),
            reason: format!(
                "no target kit version given; this build registers only {}",
                crate::kit::registry::all_kits()
                    .iter()
                    .map(|k| format!(
                        "'{}@{}'",
                        k.reference.id,
                        k.reference.version.as_deref().unwrap_or("none")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }));
    };
    let descriptor = match crate::kit::registry::inspect_kit(
        &receipt.kit,
        // The pinned version is what the project reports, not what the target
        // is; the target selector names the version to move to.
        against.strip_prefix(&format!("{}@", receipt.kit)),
    ) {
        Ok(descriptor) => descriptor,
        Err(err) => {
            return Ok(Err(KitUnavailable {
                path: dir.display().to_string(),
                reason: format!("kit upgrade target is unavailable: {err}"),
            }))
        }
    };

    let owned = read_verified_assets(&descriptor)?;
    let recorded: std::collections::BTreeMap<&str, &str> = receipt
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.digest.as_str()))
        .collect();
    let mut entries = Vec::new();
    let mut conflicts = Vec::new();
    for (path, content) in &owned {
        let on_disk = fs::read(dir.join(path)).ok();
        let before = on_disk
            .as_ref()
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
            .unwrap_or_default();
        let change = match &on_disk {
            None => KitDiffChange::Added,
            Some(bytes) if bytes == content.as_bytes() => KitDiffChange::Unchanged,
            Some(bytes) => match recorded.get(path.as_str()) {
                Some(digest) if **digest == digest_of(bytes) => KitDiffChange::Updated,
                Some(_) => KitDiffChange::Modified,
                None => KitDiffChange::Foreign,
            },
        };
        if matches!(change, KitDiffChange::Modified | KitDiffChange::Foreign) {
            conflicts.push(path.clone());
        }
        entries.push(KitDiffEntry {
            path: path.clone(),
            change,
            before,
            after: content.clone(),
        });
    }
    let targets: std::collections::BTreeSet<&str> = owned.iter().map(|(p, _)| p.as_str()).collect();
    for file in &receipt.files {
        if !targets.contains(file.path.as_str()) && dir.join(&file.path).exists() {
            entries.push(KitDiffEntry {
                path: file.path.clone(),
                change: KitDiffChange::Orphaned,
                before: fs::read_to_string(dir.join(&file.path)).unwrap_or_default(),
                after: String::new(),
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    conflicts.sort();
    Ok(Ok(KitDiffReport {
        path: dir.display().to_string(),
        project: receipt.project,
        profile: receipt.profile,
        from: format!(
            "{}@{}",
            receipt.kit,
            receipt.version.as_deref().unwrap_or("none")
        ),
        against: against.to_string(),
        entries,
        conflicts,
    }))
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

/// Apply an explicit kit upgrade.
///
/// Refuses unless `confirm` is set. An edited owned file is a conflict: the
/// upgrade writes nothing and leaves the edited file untouched, reusing the
/// same ownership-conflict code the standard receipt machinery already emits
/// rather than inventing a second ownership system. Nothing else can move a
/// pinned project: `forge new`, `forge doctor`, `forge list` and CI never call
/// this path.
pub fn upgrade_kit_snapshot(
    dir: &Path,
    to: &str,
    confirm: bool,
    force: bool,
    generated_at: &str,
) -> Result<Result<KitUpgradeReport, KitUnavailable>, ForgeError> {
    if !confirm {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "kit upgrade to '{to}' requires explicit confirmation; no files were changed"
            ),
        });
    }
    let plan = match diff_kit_snapshot(dir, Some(to))? {
        Ok(plan) => plan,
        Err(unavailable) => return Ok(Err(unavailable)),
    };
    if !plan.conflicts.is_empty() && !force {
        // The existing ownership-conflict refusal: the edited file is left
        // exactly as the operator wrote it.
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "kit upgrade to '{to}' would replace modified or foreign owned file(s): {}; \
                 the edited file was left untouched and nothing was changed",
                plan.conflicts.join(", ")
            ),
        });
    }
    let receipt = read_receipt(dir)?.expect("a diff required a receipt");
    let descriptor = crate::kit::registry::inspect_kit(
        &receipt.kit,
        to.strip_prefix(&format!("{}@", receipt.kit)),
    )
    .map_err(|err| ForgeError::KitUnknown {
        reason: err.to_string(),
    })?;
    let owned = read_verified_assets(&descriptor)?;
    let forced = plan.conflicts.clone();
    let orphaned: Vec<String> = plan
        .entries
        .iter()
        .filter(|e| e.change == KitDiffChange::Orphaned)
        .map(|e| e.path.clone())
        .collect();

    // Capture prior bytes so a failed write can be rolled back and the
    // destination is never left half-wired.
    let mut prior: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for (path, _) in &owned {
        let full = dir.join(path);
        let before = fs::read(&full).ok();
        prior.push((full, before));
    }
    let mut written = Vec::new();
    let mut apply = || -> Result<(), ForgeError> {
        for (path, content) in &owned {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).map_err(|err| ForgeError::KitDigestMismatch {
                    reason: format!("cannot create '{}': {err}", parent.display()),
                })?;
            }
            fs::write(&full, content).map_err(|err| ForgeError::KitDigestMismatch {
                reason: format!("cannot write '{}': {err}", full.display()),
            })?;
        }
        let new_receipt = render_receipt(&receipt.project, &receipt.profile, &descriptor, &owned);
        let receipt_path = dir.join(PLATFORM_RECEIPT_PATH);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&receipt_path, receipt_text(&new_receipt)?).map_err(|err| {
            ForgeError::KitDigestMismatch {
                reason: format!("cannot write '{}': {err}", receipt_path.display()),
            }
        })?;
        written = owned.iter().map(|(p, _)| p.clone()).collect();
        written.push(PLATFORM_RECEIPT_PATH.to_string());
        Ok(())
    };
    if let Err(err) = apply() {
        for (path, before) in prior.iter().rev() {
            match before {
                Some(bytes) => {
                    let _ = fs::write(path, bytes);
                }
                None => {
                    let _ = fs::remove_file(path);
                }
            }
        }
        return Err(err);
    }
    Ok(Ok(KitUpgradeReport {
        path: dir.display().to_string(),
        project: receipt.project,
        profile: receipt.profile,
        to: to.to_string(),
        written,
        forced,
        orphaned,
        generated_at: generated_at.to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Committed-feed integrity
//
// The feed is committed, so it can drift exactly the way a pinned SDK drifts
// from the runner that has to satisfy it: everything still looks correct until
// a machine that never carried the state tries to restore. These are the
// checks that turn that into a failing command instead.
// ---------------------------------------------------------------------------

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

/// Check a committed feed against a declared kit version.
///
/// Four independent ways a committed feed can be wrong, all of which have to
/// fail rather than warn:
///
/// 1. a declared package is **absent** — a feed that is merely nonempty is not
///    a feed that restores;
/// 2. a committed package's version **differs** from the declared version;
/// 3. a committed package the kit does **not** declare is present — the feed
///    has silently drifted into a different set;
/// 4. a committed byte differs from the digest recorded for it.
///
/// `declared_dir` is the directory holding the committed feed: the vendored
/// `kits/feed/` for Forge's own feed, or a generated project's
/// `packages/platform-feed/`. The declared version is checked against the file
/// names, which is where a version bump actually shows up.
pub fn verify_committed_feed(
    descriptor: &crate::kit::registry::KitDescriptor,
    declared_version: &str,
    declared_by: &str,
    feed_dir: &Path,
) -> Result<FeedVerificationReport, ForgeError> {
    let kit_id = descriptor.reference.id.as_str();
    let declared: BTreeMap<&str, &crate::kit::registry::FeedPackage> = descriptor
        .feed()
        .iter()
        .map(|e| (e.package.as_str(), e))
        .collect();
    if declared.is_empty() {
        return Err(ForgeError::KitFeedIncomplete {
            reason: format!(
                "kit '{kit_id}' declares no feed packages, so there is nothing to verify; \
                 this is a descriptor defect, not a pass"
            ),
        });
    }

    let mut committed: BTreeMap<String, String> = BTreeMap::new();
    let entries = fs::read_dir(feed_dir).map_err(|err| ForgeError::KitFeedIncomplete {
        reason: format!(
            "committed feed directory {} is unreadable: {err}; a fresh clone could not restore \
             from it",
            feed_dir.display()
        ),
    })?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".nupkg") {
            continue;
        }
        let stem = name.trim_end_matches(".nupkg");
        // The package name is itself dotted and so is the version, so the
        // version cannot be recovered by splitting on the last dot. It is
        // recovered by matching the *declared* package names as a prefix,
        // which is also the check we want: a file that matches no declared
        // package is a package this kit does not ship.
        let matched = declared
            .keys()
            .filter(|p| stem.starts_with(**p) && stem[p.len()..].starts_with('.'))
            // Longest name wins, so `Platform.Core` never claims
            // `Platform.Core.Extra.0.1.0`.
            .max_by_key(|p| p.len());
        let Some(package) = matched else {
            return Err(ForgeError::KitFeedIncomplete {
                reason: format!(
                    "committed feed file '{name}' does not match any package kit '{kit_id}' \
                     declares (declared: {}), so its version cannot be checked against the \
                     declared kit version '{declared_version}'; the feed has drifted into a \
                     different set",
                    declared.keys().copied().collect::<Vec<_>>().join(", ")
                ),
            });
        };
        committed.insert(
            (*package).to_string(),
            stem[package.len() + 1..].to_string(),
        );
    }

    // 3. Undeclared committed package: the feed has drifted into a different
    //    set, which a restore would happily accept.
    for package in committed.keys() {
        if !declared.contains_key(package.as_str()) {
            return Err(ForgeError::KitFeedIncomplete {
                reason: format!(
                    "committed feed package '{package}' is not declared by kit '{kit_id}' at \
                     version '{declared_version}' (declared: {}); the feed has drifted into a \
                     different set",
                    declared.keys().copied().collect::<Vec<_>>().join(", ")
                ),
            });
        }
    }

    let mut packages = Vec::with_capacity(declared.len());
    for (package, entry) in &declared {
        match committed.get(*package) {
            // 1. Missing.
            None => {
                return Err(ForgeError::KitFeedIncomplete {
                    reason: format!(
                        "committed feed package '{package}' is missing from {}; the kit version \
                         declared by {declared_by} is '{declared_version}', and a feed missing a \
                         package the pre-wired set needs does not restore",
                        feed_dir.display()
                    ),
                })
            }
            // 2. Version drift: the exact defect class of a pinned SDK the CI
            //    runner does not have.
            Some(version) if version != declared_version => {
                return Err(ForgeError::KitFeedVersionMismatch {
                    reason: format!(
                        "committed feed package '{package}' is at version '{version}' but {declared_by} \
                         declares kit version '{declared_version}'; run `forge kit pack` to \
                         repack the feed at the declared version"
                    ),
                })
            }
            Some(_) => packages.push(FeedVerificationEntry {
                package: package.to_string(),
                version: declared_version.to_string(),
                file: entry.file.clone(),
                role: match entry.role {
                    crate::kit::registry::FeedPackageRole::Confirmed => "confirmed",
                    crate::kit::registry::FeedPackageRole::Transitive => "transitive",
                }
                .to_string(),
                state: "present".to_string(),
            }),
        }
    }

    // 4. Tampered or unrecorded bytes. Only meaningful for the vendored feed,
    //    whose digests are recorded in `kits/manifest.json`. Every *declared*
    //    package must have a recorded digest, so a file that was dropped in
    //    without being recorded cannot pass as verified.
    if feed_dir.starts_with(kits_dir()) {
        for (package, entry) in &declared {
            let relative = entry.kits_path();
            let expected =
                manifest_digest(&relative)?.ok_or_else(|| ForgeError::KitDigestMismatch {
                    reason: format!(
                        "committed feed package '{relative}' has no digest in kits/manifest.json, \
                         so its bytes cannot be verified; run `forge kit pack` to record it. \
                         Nothing was staged and no project was registered"
                    ),
                })?;
            let bytes = fs::read(feed_dir.join(&entry.file)).map_err(|err| {
                ForgeError::KitFeedIncomplete {
                    reason: format!(
                        "committed feed package '{}' is unreadable at {}: {err}; a fresh clone \
                         could not restore it",
                        entry.file,
                        feed_dir.join(&entry.file).display()
                    ),
                }
            })?;
            let actual = digest_of(&bytes);
            if actual != expected {
                return Err(ForgeError::KitDigestMismatch {
                    reason: format!(
                        "committed feed package '{relative}' has drifted: expected {expected} got \
                         {actual}; run `forge kit pack` if the repack was intended"
                    ),
                });
            }
            let _ = package;
        }
        verify_kit_digests()?;
    }

    Ok(FeedVerificationReport {
        kit: kit_id.to_string(),
        declared_version: declared_version.to_string(),
        declared_by: declared_by.to_string(),
        feed_path: feed_dir.display().to_string(),
        packages,
    })
}

/// Read a generated project's own `forge.yaml` `kit.version` and
/// `kit.feed.path`, so a check can be made against what the project *says* it
/// pins rather than only against what Forge currently compiles in.
pub fn declared_kit_version(
    manifest: &serde_yaml::Value,
) -> Option<(Option<String>, Option<String>)> {
    let kit = manifest.get("kit")?;
    let version = kit
        .get("version")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let feed_path = kit
        .get("feed")
        .and_then(|f| f.get("path"))
        .and_then(|p| p.as_str())
        .map(str::to_string);
    Some((version, feed_path))
}

/// Write the per-file digest of every packed feed package into
/// `kits/manifest.json`, replacing any previous entry for the same path.
///
/// `forge kit pack` is the only writer. `dotnet pack` is not byte-reproducible,
/// so a repack legitimately changes every digest; recording them here keeps the
/// *version* — the thing a restore actually resolves — the compiled-in,
/// reviewed fact, while the byte-level evidence stays in one auditable file.
pub fn record_feed_digests(
    feed_dir: &Path,
    packages: &[crate::kit::registry::FeedPackage],
    source: &str,
) -> Result<usize, ForgeError> {
    let mut manifest = load_manifest()?;
    let mut written = 0usize;
    for entry in packages {
        let relative = entry.kits_path();
        let file = feed_dir.join(&entry.file);
        let bytes = fs::read(&file).map_err(|err| ForgeError::KitPackUnavailable {
            reason: format!(
                "packed feed package '{}' is unreadable at {}: {err}. The manifest was not \
                 updated, so the feed is not recorded as complete",
                entry.file,
                file.display()
            ),
        })?;
        let sha256 = digest_of(&bytes);
        match manifest.files.iter_mut().find(|f| f.path == relative) {
            Some(existing) => {
                existing.sha256 = sha256;
                existing.source_name = Some(source.to_string());
            }
            None => {
                manifest.files.push(ManifestFile {
                    path: relative,
                    sha256,
                    source_name: Some(source.to_string()),
                    revision: None,
                });
            }
        }
        written += 1;
    }
    // Stable order so a repack produces a reviewable diff rather than a
    // reshuffle.
    manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
    let path = kits_dir().join("manifest.json");
    let text =
        serde_json::to_string_pretty(&manifest).map_err(|err| ForgeError::KitPackUnavailable {
            reason: format!(
                "cannot serialise kits/manifest.json after packing: {err}. The feed bytes were \
                 written but no digest was recorded, so the feed is not accepted"
            ),
        })?;
    fs::write(&path, format!("{text}\n")).map_err(|err| ForgeError::KitPackUnavailable {
        reason: format!(
            "cannot write {}: {err}. The feed bytes were written but no digest was recorded, so \
             the feed is not accepted",
            path.display()
        ),
    })?;
    Ok(written)
}
