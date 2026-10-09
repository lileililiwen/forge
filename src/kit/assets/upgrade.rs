//! Kit assets: upgrade.

use crate::core::ForgeError;
use std::fs;
use std::path::{Path, PathBuf};

use super::model::{KitDiffChange, KitUnavailable, KitUpgradeReport, ManifestFile};
use super::paths::PLATFORM_RECEIPT_PATH;

use super::diff::diff_kit_snapshot;
use super::receipt::{read_receipt, receipt_text, render_receipt};
use super::source::{digest_of, kits_dir, load_manifest};
use super::verification::read_verified_assets;

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
    // destination is never left half-wired. The receipt and the manifest are
    // in the rollback set alongside the owned files: they are the two other
    // files this function writes, and a rollback that left either behind
    // would produce exactly the half-wired state the spec forbids.
    let receipt_path = dir.join(PLATFORM_RECEIPT_PATH);
    let manifest_path = dir.join("forge.yaml");
    let mut prior: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for (path, _) in &owned {
        let full = dir.join(path);
        let before = fs::read(&full).ok();
        prior.push((full, before));
    }
    prior.push((receipt_path.clone(), fs::read(&receipt_path).ok()));
    let manifest_before = fs::read(&manifest_path).ok();
    prior.push((manifest_path.clone(), manifest_before));
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
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&receipt_path, receipt_text(&new_receipt)?).map_err(|err| {
            ForgeError::KitDigestMismatch {
                reason: format!("cannot write '{}': {err}", receipt_path.display()),
            }
        })?;
        // The declared pin moves with the bytes. This is the last write, so a
        // manifest that cannot record the new version fails the whole upgrade
        // and the rollback below restores the owned files and the receipt.
        let target_version = descriptor
            .reference
            .version
            .clone()
            .unwrap_or_else(|| "none".to_string());
        let manifest_updated = record_declared_kit_version(dir, &target_version)?;
        written = owned.iter().map(|(p, _)| p.clone()).collect();
        written.push(PLATFORM_RECEIPT_PATH.to_string());
        if manifest_updated {
            written.push("forge.yaml".to_string());
        }
        written.sort();
        written.dedup();
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

/// Rewrite the `version:` line inside a `forge.yaml` `kit:` block, leaving
/// every other byte of the file untouched.
///
/// A targeted line edit, never a YAML round-trip. The manifest is a
/// deterministic render, and its key order, comments and layout are part of
/// the byte-identical contract; re-serializing it would silently rewrite all
/// of that to satisfy a one-field change. Returns `None` when the document
/// declares no `kit.version` to rewrite.
fn rewrite_declared_version(text: &str, version: &str) -> Option<String> {
    let rendered = format!("  version: {}\n", crate::generate::yaml_scalar(version));
    let mut out = String::with_capacity(text.len() + rendered.len());
    let mut in_kit = false;
    let mut replaced = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        // A top-level key closes the previous top-level block. `kit:` opens
        // this one; a key merely starting with the same letters, such as
        // `kits:`, does not.
        if !trimmed.is_empty() && !trimmed.starts_with(' ') {
            in_kit = trimmed.starts_with("kit:");
        }
        if in_kit && !replaced && trimmed.starts_with("  version:") {
            out.push_str(&rendered);
            replaced = true;
            continue;
        }
        out.push_str(line);
    }
    replaced.then_some(out)
}

/// Record a new pinned kit version in a generated project's own `forge.yaml`.
///
/// The project's declared pin and the bytes committed inside it have to move
/// together. Rewriting the owned files and the receipt while leaving
/// `kit.version` behind would make the project claim a version its own
/// committed feed is not, and `forge kit verify <path>` — which deliberately
/// reads what the project *says* it pins — would then fail on a project that
/// was just successfully upgraded.
///
/// Returns `true` when the declared version actually changed.
pub fn record_declared_kit_version(dir: &Path, version: &str) -> Result<bool, ForgeError> {
    let path = dir.join("forge.yaml");
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::KitFeedIncomplete {
        reason: format!(
            "cannot read {} to record the pinned kit version: {err}. The upgrade is refused, \
             because a project whose owned files and receipt moved to '{version}' while its \
             manifest still declares the old pin can no longer pass `forge kit verify`",
            path.display()
        ),
    })?;
    let Some(next) = rewrite_declared_version(&text, version) else {
        return Err(ForgeError::KitFeedIncomplete {
            reason: format!(
                "{} declares no `kit:` block `version:` line, so the upgrade cannot record the \
                 version it pins. The owned files and the receipt were left untouched and the \
                 project still reports its previous kit version",
                path.display()
            ),
        });
    };
    if next == text {
        return Ok(false);
    }
    fs::write(&path, &next).map_err(|err| ForgeError::KitFeedIncomplete {
        reason: format!(
            "cannot write {}: {err}. The upgrade is refused rather than leaving a project whose \
             manifest disagrees with its own committed feed",
            path.display()
        ),
    })?;
    Ok(true)
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
