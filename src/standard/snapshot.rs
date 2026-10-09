//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::profile::inspect_profile;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::layout::{
    BASELINE_PACK, BASELINE_PROFILES, CI_PATH, CI_TEMPLATE, COMPOSE_PATH, COMPOSE_TEMPLATE,
    DETERMINISTIC_TIMESTAMP, PROFILE_PATH, QUALITY_PATH, QUALITY_TEMPLATE, RECEIPT_PATH,
    STANDARD_SCHEMA, VERIFY_PATH,
};
use super::model::{
    DiffChange, DiffEntry, DiffReport, FileReport, FileState, PackAsset, PackAssetOrigin,
    PackDescriptor, PackEvidence, PackSupportState, Receipt, ReceiptFile, RenderedSnapshot,
    SnapshotReport, SnapshotState, UpgradeReport,
};

fn build_pack(
    id: &str,
    version: &str,
    support_state: PackSupportState,
    evidence: PackEvidence,
    compatible_profiles: &[&str],
    external_source: Option<&str>,
    files: &[(&str, &str)],
) -> PackDescriptor {
    let files: Vec<PackAsset> = files
        .iter()
        .map(|(path, template)| PackAsset {
            path: (*path).to_string(),
            template: (*template).to_string(),
        })
        .collect();
    let asset_digest = pack_asset_digest(&files);
    PackDescriptor {
        id: id.to_string(),
        version: version.to_string(),
        support_state,
        evidence,
        compatible_profiles: compatible_profiles.iter().map(|s| s.to_string()).collect(),
        external_source: external_source.map(str::to_string),
        asset_digest,
        files,
    }
}

/// SHA-256 over the canonical asset listing: paths sorted, each followed by
/// its raw template. Independent of project inputs, so it is a property of
/// the pack version alone.
pub(super) fn pack_asset_digest(files: &[PackAsset]) -> String {
    let mut sorted: Vec<&PackAsset> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hasher = Sha256::new();
    for asset in sorted {
        hasher.update(asset.path.as_bytes());
        hasher.update([0u8]);
        hasher.update(asset.template.as_bytes());
        hasher.update([0u8]);
    }
    format!("{:x}", hasher.finalize())
}

/// Hex SHA-256 of a byte string, the digest currency of every receipt.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn profile_declaration() -> &'static str {
    "schema: {schema}\npack: {pack}\nversion: {version}\nprofile: {profile}\nproject: {id}\n"
}

fn verify_script(v1_1: bool) -> &'static str {
    if v1_1 {
        "#!/usr/bin/env sh\n# Forge standard snapshot {pack}@{version} for {id}.\nset -eu\necho \"standard {pack}@{version}\"\n{test_command}\n"
    } else {
        "#!/usr/bin/env sh\n# Forge standard snapshot {pack}@{version} for {id}.\nset -eu\n{test_command}\n"
    }
}

/// All pack descriptors in stable `(id, version)` order. The baseline
/// family spans the full lifecycle so `list` can show supported,
/// deprecated and proposed states and upgrades can move between versions.
pub fn all_packs() -> Vec<PackDescriptor> {
    vec![
        build_pack(
            BASELINE_PACK,
            "0.9.0",
            PackSupportState::Deprecated,
            PackEvidence::Verified,
            BASELINE_PROFILES,
            Some("workspace-governance/templates/runtime"),
            &[
                (PROFILE_PATH, profile_declaration()),
                (VERIFY_PATH, verify_script(false)),
                (COMPOSE_PATH, COMPOSE_TEMPLATE),
            ],
        ),
        build_pack(
            BASELINE_PACK,
            "1.0.0",
            PackSupportState::Supported,
            PackEvidence::Verified,
            BASELINE_PROFILES,
            Some("workspace-governance/templates/runtime"),
            &[
                (PROFILE_PATH, profile_declaration()),
                (VERIFY_PATH, verify_script(false)),
                (CI_PATH, CI_TEMPLATE),
                (QUALITY_PATH, QUALITY_TEMPLATE),
                (COMPOSE_PATH, COMPOSE_TEMPLATE),
            ],
        ),
        build_pack(
            BASELINE_PACK,
            "1.1.0",
            PackSupportState::Supported,
            PackEvidence::Verified,
            BASELINE_PROFILES,
            Some("workspace-governance/templates/runtime"),
            &[
                (PROFILE_PATH, profile_declaration()),
                (VERIFY_PATH, verify_script(true)),
                (CI_PATH, CI_TEMPLATE),
                (QUALITY_PATH, QUALITY_TEMPLATE),
                (COMPOSE_PATH, COMPOSE_TEMPLATE),
            ],
        ),
        build_pack(
            BASELINE_PACK,
            "2.0.0",
            PackSupportState::Proposed,
            PackEvidence::Unverified,
            BASELINE_PROFILES,
            None,
            &[
                (PROFILE_PATH, profile_declaration()),
                (VERIFY_PATH, verify_script(true)),
                (CI_PATH, CI_TEMPLATE),
                (QUALITY_PATH, QUALITY_TEMPLATE),
                (COMPOSE_PATH, COMPOSE_TEMPLATE),
            ],
        ),
    ]
}

/// Parse `<pack>@<version>`. Both halves are required: there is no implicit
/// `latest`, matching the profile/feature version discipline.
pub fn parse_pack_spec(spec: &str) -> Result<(String, String), ForgeError> {
    let Some((id, version)) = spec.split_once('@') else {
        return Err(ForgeError::StandardInvalid {
            reason: format!("pack selector '{spec}' must be `<pack>@<version>`"),
        });
    };
    if id.trim().is_empty() || version.trim().is_empty() {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "pack selector '{spec}' must name both a pack and a version (`<pack>@<version>`)"
            ),
        });
    }
    Ok((id.trim().to_string(), version.trim().to_string()))
}

/// Resolve a `<pack>@<version>` selector to a descriptor, naming the
/// selector on failure. Never guesses a nearby version.
pub fn inspect_pack(spec: &str) -> Result<PackDescriptor, ForgeError> {
    let (id, version) = parse_pack_spec(spec)?;
    all_packs()
        .into_iter()
        .find(|p| p.id == id && p.version == version)
        .ok_or_else(|| ForgeError::StandardInvalid {
            reason: format!("unknown standard pack '{spec}'"),
        })
}

/// Normalize an explicit selection for generation. The selector must name a
/// supported, verified pack that is compatible with the profile; a bare flag
/// is never expanded to an implicit `latest`.
pub fn select_for_generation(profile: &str, spec: &str) -> Result<String, ForgeError> {
    let pack = inspect_pack(spec)?;
    if !pack.is_selectable() {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "standard pack '{spec}' is {} and is not selectable for new generation; \
                 choose a supported pack with verified fixtures or pin it later with \
                 `standard upgrade`; no standard files were written",
                pack_state_word(pack.support_state)
            ),
        });
    }
    if !pack.supports(profile) {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "standard pack '{spec}' does not support profile '{profile}' \
                 (compatible: {}); no standard files were written",
                pack.compatible_profiles.join(", ")
            ),
        });
    }
    Ok(spec.to_string())
}

/// Staged files generation adds when a pack is explicitly selected: the
/// owned content files plus the receipt.
pub fn staged_files(
    id: &str,
    profile: &str,
    spec: &str,
    generated_at: &str,
) -> Result<Vec<(String, String)>, ForgeError> {
    let pack = inspect_pack(spec)?;
    let snapshot = render_snapshot(id, profile, &pack, generated_at, false)?;
    let mut files = snapshot.files;
    files.push((RECEIPT_PATH.to_string(), receipt_text(&snapshot.receipt)?));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Decide the asset origin for a pack. An external source is used only when
/// it exists locally *and* carries every declared asset; otherwise Forge
/// uses its bundled local fallback. Nothing is ever fetched or invented.
pub fn resolve_asset_origin(
    pack: &PackDescriptor,
    external_dir: Option<&Path>,
) -> (PackAssetOrigin, Vec<String>) {
    let mut notes = Vec::new();
    if let Some(source) = pack.external_source.as_deref() {
        match external_dir {
            Some(dir) if dir.is_dir() && dir.join(PROFILE_PATH).is_file() => {
                return (PackAssetOrigin::External(dir.to_path_buf()), notes);
            }
            Some(dir) => notes.push(format!(
                "external template source '{source}' at '{}' is unavailable or incomplete; \
                 using the bundled local fallback (never fetched)",
                dir.display()
            )),
            None => notes.push(format!(
                "external template source '{source}' is not configured; \
                 using the bundled local fallback (never fetched)"
            )),
        }
    }
    (PackAssetOrigin::LocalFallback, notes)
}

fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

/// Render a pack for a project. Pure: identical (pack, profile, id,
/// timestamp) inputs produce identical bytes. Refuses a non-selectable pack
/// unless `allow_pinned` is set (the read/upgrade path for a pinned or
/// deprecated version).
pub fn render_snapshot(
    id: &str,
    profile: &str,
    pack: &PackDescriptor,
    generated_at: &str,
    allow_pinned: bool,
) -> Result<RenderedSnapshot, ForgeError> {
    if !pack.supports(profile) {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "standard pack '{}@{}' does not support profile '{profile}' \
                 (compatible: {}); no standard files were written",
                pack.id,
                pack.version,
                pack.compatible_profiles.join(", ")
            ),
        });
    }
    if !allow_pinned && !pack.is_selectable() {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "standard pack '{}@{}' is {} and is not selectable for new generation; \
                 pin it explicitly for an existing project; no standard files were written",
                pack.id,
                pack.version,
                pack_state_word(pack.support_state)
            ),
        });
    }
    let descriptor = inspect_profile(profile).map_err(|_| ForgeError::StandardInvalid {
        reason: format!("standard pack rendering needs a known profile, got '{profile}'"),
    })?;
    let vars: &[(&str, &str)] = &[
        ("schema", "1"),
        ("pack", pack.id.as_str()),
        ("version", pack.version.as_str()),
        ("profile", profile),
        ("id", id),
        ("build_command", descriptor.build_command.as_str()),
        ("test_command", descriptor.test_command.as_str()),
    ];
    let mut files: Vec<(String, String)> = pack
        .files
        .iter()
        .map(|asset| (asset.path.clone(), substitute(&asset.template, vars)))
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let receipt_files: Vec<ReceiptFile> = files
        .iter()
        .map(|(path, content)| ReceiptFile {
            path: path.clone(),
            digest: sha256_hex(content.as_bytes()),
        })
        .collect();
    Ok(RenderedSnapshot {
        pack: pack.id.clone(),
        version: pack.version.clone(),
        profile: profile.to_string(),
        asset_digest: pack.asset_digest.clone(),
        files,
        receipt: Receipt {
            schema: STANDARD_SCHEMA,
            generator: format!("forge@{}", env!("CARGO_PKG_VERSION")),
            project: id.to_string(),
            profile: profile.to_string(),
            pack: pack.id.clone(),
            version: pack.version.clone(),
            asset_digest: pack.asset_digest.clone(),
            generated_at: generated_at.to_string(),
            files: receipt_files,
        },
    })
}

fn pack_state_word(state: PackSupportState) -> &'static str {
    match state {
        PackSupportState::Proposed => "proposed",
        PackSupportState::Supported => "supported",
        PackSupportState::Deprecated => "deprecated",
    }
}

/// Serialize a receipt as its canonical document (pretty JSON, trailing
/// newline).
pub fn receipt_text(receipt: &Receipt) -> Result<String, ForgeError> {
    let text =
        serde_json::to_string_pretty(receipt).map_err(|err| ForgeError::StandardInvalid {
            reason: format!("standard receipt is not serializable: {err}"),
        })?;
    Ok(format!("{text}\n"))
}

fn read_receipt(dir: &Path) -> Result<Option<Receipt>, ForgeError> {
    let path = dir.join(RECEIPT_PATH);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ForgeError::StandardInvalid {
                reason: format!("cannot read standard receipt '{}': {err}", path.display()),
            })
        }
    };
    let receipt: Receipt =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::StandardInvalid {
            reason: format!(
                "standard receipt '{}' is not a valid receipt: {err}",
                path.display()
            ),
        })?;
    Ok(Some(receipt))
}

/// Observe a project's standard snapshot without changing anything. An
/// absent receipt is a reportable state, not an error.
pub fn check_snapshot(dir: &Path) -> Result<SnapshotReport, ForgeError> {
    let location = dir.display().to_string();
    let Some(receipt) = read_receipt(dir)? else {
        return Ok(SnapshotReport {
            state: SnapshotState::Absent,
            path: location,
            pack: None,
            version: None,
            profile: None,
            asset_digest: None,
            asset_digest_matches: None,
            files: Vec::new(),
            issues: vec![format!(
                "no Forge-owned standard snapshot ({RECEIPT_PATH} absent); \
                 the profile may map to no pack or generation omitted it"
            )],
        });
    };
    let mut issues = Vec::new();
    let mut files = Vec::new();
    for owned in &receipt.files {
        match fs::read(dir.join(&owned.path)) {
            Ok(bytes) => {
                if sha256_hex(&bytes) == owned.digest {
                    files.push(FileReport {
                        path: owned.path.clone(),
                        state: FileState::Present,
                    });
                } else {
                    files.push(FileReport {
                        path: owned.path.clone(),
                        state: FileState::Modified,
                    });
                }
            }
            Err(_) => files.push(FileReport {
                path: owned.path.clone(),
                state: FileState::Missing,
            }),
        }
    }
    let known = all_packs()
        .into_iter()
        .find(|p| p.id == receipt.pack && p.version == receipt.version);
    let asset_digest_matches = known
        .as_ref()
        .map(|pack| pack.asset_digest == receipt.asset_digest);
    let state = match &known {
        None => {
            issues.push(format!(
                "receipt names standard pack '{}@{}' which this build does not know; \
                 snapshot cannot be verified against a descriptor",
                receipt.pack, receipt.version
            ));
            SnapshotState::Unknown
        }
        Some(pack) => {
            if !pack.supports(&receipt.profile) {
                issues.push(format!(
                    "standard pack '{}@{}' does not list profile '{}' as compatible",
                    receipt.pack, receipt.version, receipt.profile
                ));
            }
            if asset_digest_matches == Some(false) {
                issues.push(format!(
                    "recorded asset digest does not match the descriptor for '{}@{}'",
                    receipt.pack, receipt.version
                ));
            }
            if files.iter().any(|f| f.state != FileState::Present) {
                SnapshotState::Modified
            } else {
                SnapshotState::Rendered
            }
        }
    };
    Ok(SnapshotReport {
        state,
        path: location,
        pack: Some(receipt.pack),
        version: Some(receipt.version),
        profile: Some(receipt.profile),
        asset_digest: Some(receipt.asset_digest),
        asset_digest_matches,
        files,
        issues,
    })
}

/// Plan the difference between a project's snapshot and a target pack.
/// Read-only; never writes.
pub fn diff_snapshot(dir: &Path, against: &str) -> Result<DiffReport, ForgeError> {
    let Some(receipt) = read_receipt(dir)? else {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "'{}' has no Forge-owned standard snapshot ({RECEIPT_PATH}); nothing to diff",
                dir.display()
            ),
        });
    };
    let target = inspect_pack(against)?;
    let rendered = render_snapshot(
        &receipt.project,
        &receipt.profile,
        &target,
        DETERMINISTIC_TIMESTAMP,
        true,
    )?;
    let owned: std::collections::BTreeMap<&str, &str> = receipt
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.digest.as_str()))
        .collect();
    let mut entries = Vec::new();
    let mut conflicts = Vec::new();
    for (path, content) in &rendered.files {
        let on_disk = fs::read(dir.join(path)).ok();
        let change = match on_disk {
            None => DiffChange::Added,
            Some(bytes) if bytes == content.as_bytes() => DiffChange::Unchanged,
            Some(bytes) => match owned.get(path.as_str()) {
                Some(digest) if **digest == sha256_hex(&bytes) => DiffChange::Updated,
                Some(_) => DiffChange::Modified,
                None => DiffChange::Foreign,
            },
        };
        if matches!(change, DiffChange::Modified | DiffChange::Foreign) {
            conflicts.push(path.clone());
        }
        entries.push(DiffEntry {
            path: path.clone(),
            change,
        });
    }
    let target_paths: std::collections::BTreeSet<&str> =
        rendered.files.iter().map(|(p, _)| p.as_str()).collect();
    for file in &receipt.files {
        if !target_paths.contains(file.path.as_str()) && dir.join(&file.path).exists() {
            entries.push(DiffEntry {
                path: file.path.clone(),
                change: DiffChange::Orphaned,
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(DiffReport {
        path: dir.display().to_string(),
        project: receipt.project,
        profile: receipt.profile,
        from: format!("{}@{}", receipt.pack, receipt.version),
        against: against.to_string(),
        entries,
        conflicts,
    })
}

/// Apply an explicit upgrade to a target pack version.
///
/// Refuses unless `confirm` is set. A modified owned file or an unowned
/// collision is a conflict: the upgrade writes nothing unless `force`
/// supplies the reviewed resolution. Only owned snapshot files are written;
/// unrelated files and orphaned owned files are preserved.
pub fn upgrade_snapshot(
    dir: &Path,
    to: &str,
    confirm: bool,
    force: bool,
    generated_at: &str,
) -> Result<UpgradeReport, ForgeError> {
    if !confirm {
        return Err(ForgeError::StandardInvalid {
            reason: format!("standard upgrade to '{to}' requires --confirm; no files were changed"),
        });
    }
    let plan = diff_snapshot(dir, to)?;
    if !plan.conflicts.is_empty() && !force {
        return Err(ForgeError::StandardInvalid {
            reason: format!(
                "standard upgrade to '{to}' would replace modified or foreign file(s): {}; \
                 review `forge standard diff` and re-run with --force to overwrite, \
                 nothing was changed",
                plan.conflicts.join(", ")
            ),
        });
    }
    let receipt = read_receipt(dir)?.expect("diff required a receipt");
    let target = inspect_pack(to)?;
    let rendered = render_snapshot(
        &receipt.project,
        &receipt.profile,
        &target,
        generated_at,
        true,
    )?;
    let forced: Vec<String> = plan.conflicts.clone();
    let orphaned: Vec<String> = plan
        .entries
        .iter()
        .filter(|e| e.change == DiffChange::Orphaned)
        .map(|e| e.path.clone())
        .collect();
    // Capture prior bytes so a failed write can be rolled back.
    let mut prior: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for (path, _) in &rendered.files {
        let full = dir.join(path);
        let before = fs::read(&full).ok();
        prior.push((full, before));
    }
    let mut written = Vec::new();
    let mut apply = || -> Result<(), ForgeError> {
        for (path, content) in &rendered.files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).map_err(|err| ForgeError::StandardInvalid {
                    reason: format!("cannot create '{}': {err}", parent.display()),
                })?;
            }
            fs::write(&full, content).map_err(|err| ForgeError::StandardInvalid {
                reason: format!("cannot write '{}': {err}", full.display()),
            })?;
        }
        let receipt_path = dir.join(RECEIPT_PATH);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&receipt_path, receipt_text(&rendered.receipt)?).map_err(|err| {
            ForgeError::StandardInvalid {
                reason: format!("cannot write '{}': {err}", receipt_path.display()),
            }
        })?;
        written = rendered.files.iter().map(|(p, _)| p.clone()).collect();
        written.push(RECEIPT_PATH.to_string());
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
    Ok(UpgradeReport {
        path: dir.display().to_string(),
        project: receipt.project,
        profile: receipt.profile,
        to: to.to_string(),
        written,
        forced,
        orphaned,
        generated_at: generated_at.to_string(),
    })
}
