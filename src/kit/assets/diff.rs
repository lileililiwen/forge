//! Kit assets: diff.

use crate::core::ForgeError;
use std::fs;
use std::path::Path;

use super::model::{KitDiffChange, KitDiffEntry, KitDiffReport, KitUnavailable};
use super::paths::PLATFORM_RECEIPT_PATH;

use super::receipt::read_receipt;
use super::source::digest_of;
use super::verification::read_verified_assets;

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
