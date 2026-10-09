//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::registry::Registry;
use std::fs;
use std::path::Path;

use super::contract::WORKSPACE_SYNC_JOURNAL_PROJECT;
use super::detect::{adopt_import, derive_project_id, inspect_import, sync_entry};
use super::model::{WorkspaceSyncEntry, WorkspaceSyncReport, WorkspaceSyncSummary};

/// Converge every immediate child directory of `root` into `registry` in one
/// run: manifest directories are registered, decidable directories are
/// adopted via the unchanged [`adopt_import`], already-registered
/// directories report `already` without rewriting, and every other
/// directory reports an explicit skipped/failed outcome with a reason.
///
/// Sorted by leaf name with fixed vocabulary, so reruns are deterministic.
/// One failure never aborts its siblings, and a second run over an
/// unchanged root reports everything `already`. Exactly one counts-only
/// `workspace.sync` journal row is appended (`done` iff no failure).
pub fn sync_workspace(
    registry: &mut Registry,
    root: &Path,
) -> Result<WorkspaceSyncReport, ForgeError> {
    if !root.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: root.display().to_string(),
        });
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: root.display().to_string(),
        })?;

    // Identity snapshot up front: `already` is decided by read, never by
    // blind re-adoption, and collisions fail without touching anything.
    let projects = registry.list()?;
    let id_at_path = |canonical: &str| {
        projects
            .iter()
            .find(|record| record.path == canonical)
            .map(|record| (record.id.clone(), record.profile.clone()))
    };
    let path_of_id = |id: &str| {
        projects
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.path.clone())
    };

    let mut leaves: Vec<(String, Option<std::fs::FileType>)> = Vec::new();
    let read_dir = fs::read_dir(root).map_err(|_| ForgeError::PathUnavailable {
        path: root.display().to_string(),
    })?;
    for entry in read_dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let leaf = entry.file_name().to_string_lossy().to_string();
        // Metadata that cannot be read classifies the entry as an
        // `unreadable` skip below, without any filesystem probe.
        leaves.push((leaf, entry.file_type().ok()));
    }
    leaves.sort_by(|a, b| a.0.cmp(&b.0));

    let mut entries = Vec::with_capacity(leaves.len());
    for (leaf, file_type) in &leaves {
        let file_type = match file_type {
            Some(file_type) => file_type,
            None => {
                entries.push(sync_entry(
                    leaf.clone(),
                    "skipped",
                    None,
                    None,
                    Some("unreadable".to_string()),
                ));
                continue;
            }
        };
        if leaf.starts_with('.') {
            entries.push(sync_entry(
                leaf.clone(),
                "skipped",
                None,
                None,
                Some("hidden".to_string()),
            ));
            continue;
        }
        if file_type.is_symlink() {
            entries.push(sync_entry(
                leaf.clone(),
                "skipped",
                None,
                None,
                Some("symlink".to_string()),
            ));
            continue;
        }
        let dir = root.join(leaf);
        if !file_type.is_dir() || !dir.is_dir() {
            entries.push(sync_entry(
                leaf.clone(),
                "skipped",
                None,
                None,
                Some("not-a-directory".to_string()),
            ));
            continue;
        }
        let canonical = match dir.canonicalize() {
            Ok(canonical) => canonical.display().to_string(),
            Err(_) => {
                entries.push(sync_entry(
                    leaf.clone(),
                    "failed",
                    None,
                    None,
                    Some("path-unavailable".to_string()),
                ));
                continue;
            }
        };
        if !Path::new(&canonical).starts_with(&canonical_root) {
            entries.push(sync_entry(
                leaf.clone(),
                "failed",
                None,
                None,
                Some("outside-root".to_string()),
            ));
            continue;
        }
        entries.push(sync_one_directory(
            registry,
            &dir,
            leaf,
            &canonical,
            &id_at_path,
            &path_of_id,
        ));
    }

    let mut summary = WorkspaceSyncSummary {
        ok: 0,
        already: 0,
        skipped: 0,
        failed: 0,
    };
    for entry in &entries {
        match entry.outcome.as_str() {
            "ok" => summary.ok += 1,
            "already" => summary.already += 1,
            "skipped" => summary.skipped += 1,
            _ => summary.failed += 1,
        }
    }
    let state = if summary.failed == 0 {
        "done"
    } else {
        "failed"
    };
    registry.record_operation(
        "workspace.sync",
        WORKSPACE_SYNC_JOURNAL_PROJECT,
        state,
        &format!(
            "synced {}, already {}, skipped {}, failed {}",
            summary.ok, summary.already, summary.skipped, summary.failed
        ),
    )?;

    Ok(WorkspaceSyncReport {
        root: canonical_root.display().to_string(),
        entries,
        summary,
    })
}

/// Decide and converge one validated child directory. Manifest directories
/// register; decidable directories adopt; anything already registered at
/// the same canonical path reports `already` without a Core call; identity
/// collisions fail before anything is touched.
fn sync_one_directory(
    registry: &mut Registry,
    dir: &Path,
    leaf: &str,
    canonical: &str,
    id_at_path: &dyn Fn(&str) -> Option<(String, String)>,
    path_of_id: &dyn Fn(&str) -> Option<String>,
) -> WorkspaceSyncEntry {
    if dir.join("forge.yaml").is_file() {
        let (manifest, _) = match crate::core::manifest::Manifest::load_from_dir(dir, None) {
            Ok(loaded) => loaded,
            Err(err) => {
                return sync_entry(
                    leaf.to_string(),
                    "failed",
                    None,
                    None,
                    Some(err.code().to_string()),
                );
            }
        };
        let id = manifest.project.id.clone();
        let profile = manifest.project.profile.clone();
        return match (id_at_path(canonical), path_of_id(&id)) {
            (Some((owner, _)), _) if owner == id => {
                sync_entry(leaf.to_string(), "already", Some(id), Some(profile), None)
            }
            (_, Some(owner_path)) if owner_path != canonical => sync_entry(
                leaf.to_string(),
                "failed",
                Some(id),
                Some(profile),
                Some("id-collision".to_string()),
            ),
            (Some(_), _) => sync_entry(
                leaf.to_string(),
                "failed",
                Some(id),
                Some(profile),
                Some("path-collision".to_string()),
            ),
            _ => match registry.register(dir, None) {
                Ok(record) => sync_entry(
                    leaf.to_string(),
                    "ok",
                    Some(record.id),
                    Some(record.profile),
                    None,
                ),
                Err(err) => sync_entry(
                    leaf.to_string(),
                    "failed",
                    Some(id),
                    Some(profile),
                    Some(err.code().to_string()),
                ),
            },
        };
    }

    let proposal = match inspect_import(dir, None) {
        Ok(proposal) => proposal,
        Err(err) if err.code() == "ambiguous-import" => {
            return sync_entry(
                leaf.to_string(),
                "skipped",
                None,
                None,
                Some("ambiguous".to_string()),
            );
        }
        Err(err) => {
            return sync_entry(
                leaf.to_string(),
                "failed",
                None,
                None,
                Some(err.code().to_string()),
            );
        }
    };
    let profile = match proposal.suggested_profile.clone() {
        Some(profile) => profile,
        None => {
            return sync_entry(
                leaf.to_string(),
                "skipped",
                None,
                None,
                Some("undecidable".to_string()),
            );
        }
    };
    let id = match derive_project_id(dir, None) {
        Ok(id) => id,
        Err(err) => {
            return sync_entry(
                leaf.to_string(),
                "failed",
                None,
                Some(profile),
                Some(err.code().to_string()),
            );
        }
    };
    match (id_at_path(canonical), path_of_id(&id)) {
        (Some((owner, _)), _) if owner == id => {
            sync_entry(leaf.to_string(), "already", Some(id), Some(profile), None)
        }
        (_, Some(owner_path)) if owner_path != canonical => sync_entry(
            leaf.to_string(),
            "failed",
            Some(id),
            Some(profile),
            Some("id-collision".to_string()),
        ),
        (Some(_), _) => sync_entry(
            leaf.to_string(),
            "failed",
            Some(id),
            Some(profile),
            Some("path-collision".to_string()),
        ),
        _ => match adopt_import(registry, dir, None, None) {
            Ok(record) => sync_entry(
                leaf.to_string(),
                "ok",
                Some(record.id),
                Some(record.profile),
                None,
            ),
            Err(err) => sync_entry(
                leaf.to_string(),
                "failed",
                Some(id),
                Some(profile),
                Some(err.code().to_string()),
            ),
        },
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::import::detect::build_manifest_text;
    use crate::import::model::FieldStatus;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    fn rust_fixture(dir: &Path) {
        write(
            dir,
            "Cargo.toml",
            "[package]\nname = \"demo\"\n[dependencies]\naxum = \"0.7\"\nsqlx = { version = \"0.8\", features = [\"postgres\"] }\njsonwebtoken = \"9\"\n",
        );
        write(dir, "src/main.rs", "fn main() {}\n");
        write(dir, "Dockerfile", "FROM rust\n");
        write(dir, ".github/workflows/ci.yml", "on: push\n");
    }

    #[test]
    fn detects_rust_stack_with_evidence() {
        let tmp = TempDir::new().unwrap();
        rust_fixture(tmp.path());
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert_eq!(proposal.suggested_maturity.as_deref(), Some("L1"));
        assert_eq!(proposal.language.value.as_deref(), Some("rust"));
        assert_eq!(proposal.framework.value.as_deref(), Some("axum"));
        assert_eq!(proposal.package_manager.value.as_deref(), Some("cargo"));
        assert!(proposal
            .database
            .value
            .as_deref()
            .unwrap()
            .contains("postgresql"));
        assert_eq!(proposal.docker.status, FieldStatus::Detected);
        assert_eq!(proposal.ci.status, FieldStatus::Detected);
        assert_eq!(proposal.auth.status, FieldStatus::Detected);
        // No git repo in tempdir: unknown, not missing.
        assert_eq!(proposal.git_remote.status, FieldStatus::Unknown);
        assert_eq!(proposal.driftwatch.status, FieldStatus::Missing);
    }

    #[test]
    fn missing_remote_and_driftwatch_do_not_block_inspection() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert_eq!(proposal.git_remote.status, FieldStatus::Unknown);
        assert_eq!(proposal.driftwatch.status, FieldStatus::Missing);
    }

    #[test]
    fn mixed_frameworks_are_ambiguous_and_selectable() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        write(
            tmp.path(),
            "pubspec.yaml",
            "name: demo\nenvironment:\n  flutter: 3.22\n",
        );
        let err = inspect_import(tmp.path(), None).expect_err("mixed stacks must be ambiguous");
        assert_eq!(err.code(), "ambiguous-import");
        assert!(err.to_string().contains("--profile"), "{err}");
        // Explicit selection resolves without writes.
        let before: Vec<PathBuf> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        let proposal = inspect_import(tmp.path(), Some("rust-web")).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert!(proposal.explicit_profile);
        let after: Vec<PathBuf> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn monorepo_roots_are_ambiguous() {
        let tmp = TempDir::new().unwrap();
        let rust_sub = tmp.path().join("backend");
        let app_sub = tmp.path().join("mobile");
        fs::create_dir(&rust_sub).unwrap();
        fs::create_dir(&app_sub).unwrap();
        write(&rust_sub, "Cargo.toml", "[package]\nname = \"backend\"\n");
        write(
            &app_sub,
            "pubspec.yaml",
            "name: mobile\nenvironment:\n  flutter: 3.22\n",
        );
        let err = inspect_import(tmp.path(), None).expect_err("monorepo must be ambiguous");
        assert_eq!(err.code(), "ambiguous-import");
        assert!(err.to_string().contains("backend"), "{err}");
    }

    #[test]
    fn empty_directory_has_unknown_profile() {
        let tmp = TempDir::new().unwrap();
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile, None);
        assert_eq!(proposal.confidence, "none");
        assert_eq!(proposal.language.status, FieldStatus::Unknown);
    }

    #[test]
    fn derives_kebab_ids_and_rejects_garbage() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("My_Cool App!");
        fs::create_dir(&dir).unwrap();
        assert_eq!(derive_project_id(&dir, None).unwrap(), "my-cool-app");
        assert_eq!(derive_project_id(&dir, Some("ok-id")).unwrap(), "ok-id");
        assert_eq!(
            derive_project_id(&dir, Some("Bad_ID")).unwrap_err().code(),
            "import-conflict"
        );
    }

    #[test]
    fn generated_manifest_is_minimal_and_valid() {
        let text = build_manifest_text("demo-app", "rust-web", Some("rust"));
        let manifest =
            crate::core::manifest::Manifest::parse(Path::new("forge.yaml"), text.as_bytes())
                .expect("generated manifest must validate");
        assert_eq!(manifest.project.id, "demo-app");
        assert!(manifest.features.is_empty());
        crate::profile::resolve_profile(&manifest.project.profile, &[]).unwrap();
    }

    #[test]
    fn unknown_explicit_profile_fails_read_only() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let err =
            inspect_import(tmp.path(), Some("not-a-real-profile")).expect_err("unknown profile");
        assert_eq!(err.code(), "unknown-profile");
    }
}
