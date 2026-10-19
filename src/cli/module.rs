//! Local developer workflow facade for Forge-owned modules.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::{Format, Output};
use forge::core::{validate_project_id, ForgeError};

#[derive(Debug, Clone, Serialize)]
struct ModuleEntry {
    id: String,
    stack: String,
    profile: String,
    path: String,
    #[serde(skip)]
    absolute_path: PathBuf,
}

fn catalog_error(path: &Path, reason: impl std::fmt::Display) -> ForgeError {
    ForgeError::ModuleCatalogInvalid {
        reason: format!("{}: {reason}", path.display()),
    }
}

fn required_string<'a>(
    value: &'a Value,
    field: &str,
    manifest_path: &Path,
) -> Result<&'a str, ForgeError> {
    value[field]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| catalog_error(manifest_path, format!("missing non-empty `{field}`")))
}

fn discover_modules_from(root: &Path) -> Result<Vec<ModuleEntry>, ForgeError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| ForgeError::ModuleCatalogUnavailable {
            path: root.display().to_string(),
        })?;
    let modules_root = canonical_root.join("modules");
    let canonical_modules_root =
        modules_root
            .canonicalize()
            .map_err(|_| ForgeError::ModuleCatalogUnavailable {
                path: modules_root.display().to_string(),
            })?;
    if !canonical_modules_root.is_dir() {
        return Err(ForgeError::ModuleCatalogUnavailable {
            path: modules_root.display().to_string(),
        });
    }
    if canonical_modules_root.parent() != Some(canonical_root.as_path()) {
        return Err(catalog_error(
            &modules_root,
            "modules directory resolves outside the Forge checkout",
        ));
    }

    let entries = fs::read_dir(&canonical_modules_root)
        .map_err(|error| catalog_error(&canonical_modules_root, error))?;
    let mut modules = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| catalog_error(&canonical_modules_root, error))?;
        let candidate = entry.path();
        if !candidate.is_dir() {
            continue;
        }
        let canonical = candidate
            .canonicalize()
            .map_err(|error| catalog_error(&candidate, error))?;
        if canonical.parent() != Some(canonical_modules_root.as_path()) {
            return Err(catalog_error(
                &candidate,
                "module path resolves outside the immediate modules directory",
            ));
        }

        let manifest_path = canonical.join(".project.json");
        if !manifest_path.is_file() {
            continue;
        }
        let canonical_manifest = manifest_path
            .canonicalize()
            .map_err(|error| catalog_error(&manifest_path, error))?;
        if !canonical_manifest.starts_with(&canonical) {
            return Err(catalog_error(
                &manifest_path,
                "project declaration resolves outside its module directory",
            ));
        }
        let raw = fs::read(&canonical_manifest)
            .map_err(|error| catalog_error(&canonical_manifest, error))?;
        let manifest: Value = serde_json::from_slice(&raw)
            .map_err(|error| catalog_error(&canonical_manifest, error))?;
        let id = required_string(&manifest, "id", &canonical_manifest)?.to_string();
        validate_project_id(&id).map_err(|reason| catalog_error(&canonical_manifest, reason))?;
        let stack = required_string(&manifest, "stack", &canonical_manifest)?.to_string();
        let profile = required_string(&manifest, "profile", &canonical_manifest)?.to_string();
        let gate_paths = [
            canonical.join("gate.toml"),
            canonical.join(".ai-gate/gate.yaml"),
        ];
        let mut has_gate = false;
        for gate_path in gate_paths {
            if !gate_path.is_file() {
                continue;
            }
            let canonical_gate = gate_path
                .canonicalize()
                .map_err(|error| catalog_error(&gate_path, error))?;
            if !canonical_gate.starts_with(&canonical) {
                return Err(catalog_error(
                    &gate_path,
                    "Gate declaration resolves outside its module directory",
                ));
            }
            has_gate = true;
        }
        if !has_gate {
            return Err(catalog_error(
                &canonical_manifest,
                "declared module has no gate.toml or .ai-gate/gate.yaml",
            ));
        }
        let relative = candidate
            .strip_prefix(&canonical_root)
            .unwrap_or(&candidate)
            .to_string_lossy()
            .replace('\\', "/");
        modules.push(ModuleEntry {
            id,
            stack,
            profile,
            path: relative,
            absolute_path: canonical,
        });
    }

    modules.sort_by(|left, right| left.id.cmp(&right.id));
    for pair in modules.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(ForgeError::ModuleCatalogInvalid {
                reason: format!(
                    "duplicate module id '{}' at '{}' and '{}'",
                    pair[0].id, pair[0].path, pair[1].path
                ),
            });
        }
    }
    Ok(modules)
}

fn discover_modules() -> Result<Vec<ModuleEntry>, ForgeError> {
    let root =
        super::use_cmd::workspace_root().ok_or_else(|| ForgeError::ModuleCatalogUnavailable {
            path: std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "current executable".to_string()),
        })?;
    discover_modules_from(&root)
}

pub(crate) fn resolve_module(id: &str) -> Result<PathBuf, ForgeError> {
    validate_project_id(id).map_err(|reason| ForgeError::ModuleNotFound {
        id: format!("{id} ({reason})"),
    })?;
    discover_modules()?
        .into_iter()
        .find(|module| module.id == id)
        .map(|module| module.absolute_path)
        .ok_or_else(|| ForgeError::ModuleNotFound { id: id.to_string() })
}

pub(crate) fn cmd_module_list(format: Format) -> Result<Output, ForgeError> {
    let modules = discover_modules()?;
    let json = serde_json::to_value(&modules).expect("module entries serialize");
    let text = if modules.is_empty() {
        "No Forge modules with Gate declarations found.".to_string()
    } else {
        let rows = modules
            .iter()
            .map(|module| {
                format!(
                    "{:<32} {:<12} {:<24} {}",
                    module.id, module.stack, module.profile, module.path
                )
            })
            .collect::<Vec<_>>();
        format!(
            "ID                               STACK        PROFILE                  PATH\n{}",
            rows.join("\n")
        )
    };
    Ok(match format {
        Format::Human | Format::Table => Output::Human(text),
        Format::Json => Output::Json(json),
        Format::Ndjson => Output::Raw(
            modules
                .iter()
                .map(|module| serde_json::to_string(module).expect("module entry serializes"))
                .collect::<Vec<_>>()
                .join("\n")
                + if modules.is_empty() { "" } else { "\n" },
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn module(root: &Path, directory: &str, id: &str) {
        let path = root.join("modules").join(directory);
        fs::create_dir_all(path.join(".ai-gate")).unwrap();
        fs::write(
            path.join(".project.json"),
            format!(r#"{{"id":"{id}","stack":"rust","profile":"test"}}"#),
        )
        .unwrap();
        fs::write(path.join(".ai-gate/gate.yaml"), "version: 1\n").unwrap();
    }

    #[test]
    fn discovery_sorts_declared_modules_and_ignores_unregistered_directories() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("modules/unregistered")).unwrap();
        module(temp.path(), "zeta", "zeta");
        module(temp.path(), "alpha", "alpha");
        let modules = discover_modules_from(temp.path()).unwrap();
        assert_eq!(
            modules
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn discovery_allows_an_empty_module_directory_and_requires_the_root() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("modules")).unwrap();
        assert!(discover_modules_from(temp.path()).unwrap().is_empty());
        assert!(matches!(
            discover_modules_from(temp.path().join("missing-root").as_path()),
            Err(ForgeError::ModuleCatalogUnavailable { .. })
        ));
    }

    #[test]
    fn discovery_rejects_duplicate_ids_malformed_metadata_and_missing_gate() {
        let temp = tempdir().unwrap();
        module(temp.path(), "one", "same");
        module(temp.path(), "two", "same");
        assert!(matches!(
            discover_modules_from(temp.path()),
            Err(ForgeError::ModuleCatalogInvalid { .. })
        ));

        let other = tempdir().unwrap();
        let malformed = other.path().join("modules/bad");
        fs::create_dir_all(&malformed).unwrap();
        fs::write(malformed.join(".project.json"), "{").unwrap();
        assert!(matches!(
            discover_modules_from(other.path()),
            Err(ForgeError::ModuleCatalogInvalid { .. })
        ));

        let no_gate = tempdir().unwrap();
        let path = no_gate.path().join("modules/no-gate");
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join(".project.json"),
            r#"{"id":"no-gate","stack":"rust","profile":"test"}"#,
        )
        .unwrap();
        assert!(matches!(
            discover_modules_from(no_gate.path()),
            Err(ForgeError::ModuleCatalogInvalid { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn discovery_rejects_symlinks_outside_modules() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("modules")).unwrap();
        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        module(temp.path(), "inside", "inside");
        symlink(&outside, temp.path().join("modules/escape")).unwrap();
        assert!(matches!(
            discover_modules_from(temp.path()),
            Err(ForgeError::ModuleCatalogInvalid { .. })
        ));
    }
}
