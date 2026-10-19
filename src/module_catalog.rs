//! Validated discovery of modules owned by the Forge source checkout.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{validate_project_id, ForgeError};

#[derive(Debug, Clone, Serialize)]
pub struct ModuleEntry {
    pub id: String,
    pub stack: String,
    pub profile: String,
    pub path: String,
    #[serde(skip)]
    pub absolute_path: PathBuf,
}

fn invalid(path: &Path, reason: impl std::fmt::Display) -> ForgeError {
    ForgeError::ModuleCatalogInvalid {
        reason: format!("{}: {reason}", path.display()),
    }
}

fn required<'a>(value: &'a Value, field: &str, path: &Path) -> Result<&'a str, ForgeError> {
    value[field]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| invalid(path, format!("missing non-empty `{field}`")))
}

pub fn discover_from(root: &Path) -> Result<Vec<ModuleEntry>, ForgeError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| ForgeError::ModuleCatalogUnavailable {
            path: root.display().to_string(),
        })?;
    let modules_root = canonical_root.join("modules");
    let canonical_modules =
        modules_root
            .canonicalize()
            .map_err(|_| ForgeError::ModuleCatalogUnavailable {
                path: modules_root.display().to_string(),
            })?;
    if !canonical_modules.is_dir() || canonical_modules.parent() != Some(canonical_root.as_path()) {
        return Err(invalid(
            &modules_root,
            "modules directory is outside the Forge checkout",
        ));
    }
    let mut modules = Vec::new();
    for entry in fs::read_dir(&canonical_modules).map_err(|e| invalid(&canonical_modules, e))? {
        let candidate = entry.map_err(|e| invalid(&canonical_modules, e))?.path();
        if !candidate.is_dir() {
            continue;
        }
        let canonical = candidate
            .canonicalize()
            .map_err(|e| invalid(&candidate, e))?;
        if canonical.parent() != Some(canonical_modules.as_path()) {
            return Err(invalid(
                &candidate,
                "module resolves outside the immediate modules directory",
            ));
        }
        let manifest_path = canonical.join(".project.json");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest_path = manifest_path
            .canonicalize()
            .map_err(|e| invalid(&manifest_path, e))?;
        if !manifest_path.starts_with(&canonical) {
            return Err(invalid(
                &manifest_path,
                "project declaration escapes module",
            ));
        }
        let manifest: Value = serde_json::from_slice(
            &fs::read(&manifest_path).map_err(|e| invalid(&manifest_path, e))?,
        )
        .map_err(|e| invalid(&manifest_path, e))?;
        let id = required(&manifest, "id", &manifest_path)?.to_string();
        validate_project_id(&id).map_err(|e| invalid(&manifest_path, e))?;
        let stack = required(&manifest, "stack", &manifest_path)?.to_string();
        let profile = required(&manifest, "profile", &manifest_path)?.to_string();
        let gate_paths = [
            canonical.join("gate.toml"),
            canonical.join(".ai-gate/gate.yaml"),
        ];
        let mut has_gate = false;
        for path in gate_paths {
            if path.is_file() {
                let resolved = path.canonicalize().map_err(|e| invalid(&path, e))?;
                if !resolved.starts_with(&canonical) {
                    return Err(invalid(&path, "Gate declaration escapes module"));
                }
                has_gate = true;
            }
        }
        if !has_gate {
            return Err(invalid(
                &manifest_path,
                "declared module has no Gate declaration",
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
    modules.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(pair) = modules.windows(2).find(|w| w[0].id == w[1].id) {
        return Err(ForgeError::ModuleCatalogInvalid {
            reason: format!("duplicate module id '{}'", pair[0].id),
        });
    }
    Ok(modules)
}

pub fn source_root() -> Result<PathBuf, ForgeError> {
    let exe = std::env::current_exe().map_err(|_| ForgeError::ModuleCatalogUnavailable {
        path: "current executable".into(),
    })?;
    for parent in exe.ancestors() {
        if parent.join("Cargo.toml").is_file() && parent.join("modules").is_dir() {
            return Ok(parent.to_path_buf());
        }
    }
    Err(ForgeError::ModuleCatalogUnavailable {
        path: exe.display().to_string(),
    })
}

pub fn discover() -> Result<Vec<ModuleEntry>, ForgeError> {
    discover_from(&source_root()?)
}

pub fn resolve(id: &str) -> Result<ModuleEntry, ForgeError> {
    validate_project_id(id).map_err(|e| ForgeError::ModuleNotFound {
        id: format!("{id} ({e})"),
    })?;
    discover()?
        .into_iter()
        .find(|module| module.id == id)
        .ok_or_else(|| ForgeError::ModuleNotFound { id: id.to_string() })
}
