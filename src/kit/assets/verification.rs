//! Kit assets: verification.

use crate::core::ForgeError;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::model::{AssetState, FeedVerificationEntry, FeedVerificationReport, KitAssetReport};
use super::paths::PLATFORM_TOKENS_DIR;

use super::source::{digest_of, kits_dir, load_manifest};

/// The digest-pinned vendored assets of the `platform-ui-web` kit, with the
/// path each one takes inside a generated project.
///
/// Compiled in, so the descriptor is a property of the kit version alone and
/// identical inputs always render identical bytes.
pub fn token_assets() -> Vec<crate::kit::registry::AssetRef> {
    vec![
        crate::kit::registry::AssetRef {
            path: "tokens/tokens.css".to_string(),
            sha256: "7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/tokens.css"),
            binary: false,
        },
        crate::kit::registry::AssetRef {
            path: "tokens/tokens.ts".to_string(),
            sha256: "7e0633d3a18ad6039f160838412ddb5d3f30260fa8ef594d8b266f9e5d646c29".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/tokens.ts"),
            binary: false,
        },
        crate::kit::registry::AssetRef {
            path: "scripts/verify-tokens.mjs".to_string(),
            sha256: "1cba69d465cc8ed639a958f87dda03241333576694c2942dd50d6c9de4866f4b".to_string(),
            target: format!("{PLATFORM_TOKENS_DIR}/verify-tokens.mjs"),
            binary: false,
        },
    ]
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

/// Every digest recorded in `kits/manifest.json`, as `vendored path -> sha256`.
///
/// Exposed so a caller can verify a feed against an explicit digest record
/// rather than against ambient process state, which is what makes the check
/// testable without repointing the whole loader at a temporary tree.
pub fn committed_feed_digests() -> Result<BTreeMap<String, String>, ForgeError> {
    Ok(load_manifest()?
        .files
        .iter()
        .map(|f| (f.path.clone(), f.sha256.clone()))
        .collect())
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
    // The vendored `kits/` tree is the one place a digest is recorded per
    // packed file, so that is the one feed whose bytes can be verified. A
    // generated project commits its own `.nupkg` bytes and records no digest
    // for them; its feed is version-checked, and the report says so rather than
    // implying a byte-level check that did not run.
    let digests = if feed_dir.starts_with(kits_dir()) {
        Some(committed_feed_digests()?)
    } else {
        None
    };
    let report = verify_committed_feed_with_digests(
        descriptor,
        declared_version,
        declared_by,
        feed_dir,
        digests.as_ref(),
    )?;
    if digests.is_some() {
        // The whole vendored tree, not just the feed: the token artifacts ship
        // from the same manifest and drift there is the same defect.
        verify_kit_digests()?;
    }
    Ok(report)
}

/// [`verify_committed_feed`] against an explicit digest record.
///
/// The record is a parameter rather than ambient process state so the check can
/// be pointed at a temporary feed without repointing the whole vendored-asset
/// loader at it. `None` means no digest is recorded for this feed, and the
/// returned report carries `digests_verified: false`.
pub fn verify_committed_feed_with_digests(
    descriptor: &crate::kit::registry::KitDescriptor,
    declared_version: &str,
    declared_by: &str,
    feed_dir: &Path,
    digests: Option<&BTreeMap<String, String>>,
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

    // 4. Tampered or unrecorded bytes, checked only where a digest is actually
    //    recorded for the package. `digests_verified` records which happened,
    //    so a skipped byte check can never read as a clean bill of health.
    let mut digests_verified = false;
    if let Some(record) = digests {
        digests_verified = true;
        for (package, entry) in &declared {
            let relative = entry.kits_path();
            let expected = record
                .get(&relative)
                .ok_or_else(|| ForgeError::KitDigestMismatch {
                    reason: format!(
                        "committed feed package '{relative}' has no recorded digest, so its bytes \
                     cannot be verified; run `forge kit pack` to record it. Nothing was staged \
                     and no project was registered"
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
            if &actual != expected {
                return Err(ForgeError::KitDigestMismatch {
                    reason: format!(
                        "committed feed package '{relative}' has drifted: expected {expected} got \
                         {actual}; run `forge kit pack` if the repack was intended"
                    ),
                });
            }
            let _ = package;
        }
    }

    Ok(FeedVerificationReport {
        kit: kit_id.to_string(),
        declared_version: declared_version.to_string(),
        declared_by: declared_by.to_string(),
        digests_verified,
        feed_path: feed_dir.display().to_string(),
        packages,
    })
}
