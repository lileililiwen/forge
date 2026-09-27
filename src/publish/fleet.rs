//! Fleet publish for the workspace governance registry
//! (`jenkins-publish-integration`).
//!
//! Reads the workspace-governance `projects.json` document,
//! filters by lifecycle and docker-compose presence, and publishes
//! every eligible project. Core orchestrator logic lives in
//! [`super`]; this module is the registry bridge.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::ForgeError;

/// One entry from the workspace-governance `projects.json`. The
/// registry is owned by the workspace-governance sibling; we read
/// only the fields we need.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<String>,
    #[serde(default)]
    pub adoption: Option<String>,
}

/// Top-level shape of `projects.json`. The registry carries other
/// fields we don't read; missing fields default to `None`.
#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub workspace_root: Option<String>,
    pub projects: Vec<RegistryEntry>,
}

/// A project eligible for publishing: has an id, a resolvable path,
/// the requested lifecycle, and a docker-compose file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleProject {
    pub id: String,
    pub path: PathBuf,
    pub profile: Option<String>,
}

impl EligibleProject {
    pub fn compose_file(&self) -> Option<&'static str> {
        for cand in ["docker-compose.yml", "compose.yml", "compose.yaml"] {
            if self.path.join(cand).is_file() {
                return Some(cand);
            }
        }
        None
    }
}

/// Load the registry JSON from disk. Refuses on missing file,
/// unreadable file, or malformed JSON; the typed error names the
/// specific failure so the CLI can surface it to the operator.
pub fn load_registry(path: &Path) -> Result<Registry, ForgeError> {
    let bytes = std::fs::read(path).map_err(|err| ForgeError::PublishInvalid {
        reason: format!("cannot read registry {}: {err}", path.display()),
    })?;
    serde_json::from_slice::<Registry>(&bytes).map_err(|err| ForgeError::PublishInvalid {
        reason: format!(
            "registry {} is not valid JSON: {err}",
            path.display()
        ),
    })
}

/// Default registry path: `$FORGE_WORKSPACE_REGISTRY` or
/// `<workspace-governance>/projects.json` relative to the project
/// root.
pub fn default_registry_path(workspace_root: Option<&Path>) -> PathBuf {
    if let Ok(env_path) = std::env::var("FORGE_WORKSPACE_REGISTRY") {
        let trimmed = env_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let root = workspace_root
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::var("FORGE_WORKSPACE_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/home/paul/code"));
    root.join("workspace-governance/projects.json")
}

/// Default workspace root: `$FORGE_WORKSPACE_ROOT` or
/// `/home/paul/code`.
pub fn default_workspace_root() -> PathBuf {
    if let Ok(env_root) = std::env::var("FORGE_WORKSPACE_ROOT") {
        let trimmed = env_root.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from("/home/paul/code")
}

/// Filter the registry to projects that should be published.
///
/// Eligibility rules:
/// - lifecycle matches `lifecycle` (default: `active`)
/// - id is non-empty
/// - path resolves to an existing directory
/// - directory contains one of `docker-compose.yml`, `compose.yml`,
///   `compose.yaml`
pub fn filter_eligible(
    registry: &Registry,
    workspace_root: &Path,
    lifecycle: &str,
) -> Vec<EligibleProject> {
    let mut out: Vec<EligibleProject> = Vec::new();
    for entry in &registry.projects {
        if entry.lifecycle.as_deref() != Some(lifecycle) {
            continue;
        }
        let Some(path) = entry.path.as_deref().or(Some(entry.id.as_str())) else {
            continue;
        };
        let candidate = workspace_root.join(path);
        if !candidate.is_dir() {
            continue;
        }
        let eligible = EligibleProject {
            id: entry.id.clone(),
            path: candidate,
            profile: entry.profile.clone(),
        };
        if eligible.compose_file().is_none() {
            continue;
        }
        out.push(eligible);
    }
    // Stable, deterministic order so the operator can predict the
    // publish sequence.
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn registry_with(projects: Vec<(&str, &str, &str)>) -> Registry {
        Registry {
            workspace_root: None,
            projects: projects
                .into_iter()
                .map(|(id, path, lifecycle)| RegistryEntry {
                    id: id.to_string(),
                    path: Some(path.to_string()),
                    profile: Some("test".to_string()),
                    lifecycle: Some(lifecycle.to_string()),
                    adoption: Some("adopted".to_string()),
                })
                .collect(),
        }
    }

    #[test]
    fn load_registry_refuses_missing_file() {
        let err = load_registry(Path::new("/no/such/file.json")).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_registry_parses_minimal_doc() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("projects.json");
        std::fs::write(
            &path,
            r#"{"schema_version": 1, "projects": [{"id": "demo", "path": "demo"}]}"#,
        )
        .unwrap();
        let reg = load_registry(&path).unwrap();
        assert_eq!(reg.projects.len(), 1);
        assert_eq!(reg.projects[0].id, "demo");
    }

    #[test]
    fn load_registry_refuses_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "not json at all").unwrap();
        let err = load_registry(&path).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn filter_eligible_keeps_only_active_with_compose() {
        let tmp = TempDir::new().unwrap();
        let active = tmp.path().join("active");
        std::fs::create_dir(&active).unwrap();
        std::fs::write(active.join("docker-compose.yml"), "services: {}").unwrap();
        let planning = tmp.path().join("planning");
        std::fs::create_dir(&planning).unwrap();
        std::fs::write(planning.join("docker-compose.yml"), "services: {}").unwrap();
        let no_compose = tmp.path().join("nocompose");
        std::fs::create_dir(&no_compose).unwrap();

        let reg = registry_with(vec![
            (
                "active-proj",
                active.strip_prefix(tmp.path()).unwrap().to_str().unwrap(),
                "active",
            ),
            (
                "planning-proj",
                planning.strip_prefix(tmp.path()).unwrap().to_str().unwrap(),
                "planning",
            ),
            (
                "nocompose-proj",
                no_compose.strip_prefix(tmp.path()).unwrap().to_str().unwrap(),
                "active",
            ),
        ]);
        let eligible = filter_eligible(&reg, tmp.path(), "active");
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].id, "active-proj");
        assert_eq!(eligible[0].compose_file(), Some("docker-compose.yml"));
    }

    #[test]
    fn filter_eligible_sorts_by_id() {
        let tmp = TempDir::new().unwrap();
        for id in ["zeta", "alpha", "mu"] {
            let p = tmp.path().join(id);
            std::fs::create_dir(&p).unwrap();
            std::fs::write(p.join("docker-compose.yml"), "x").unwrap();
        }
        let reg = registry_with(vec![
            ("zeta", "zeta", "active"),
            ("alpha", "alpha", "active"),
            ("mu", "mu", "active"),
        ]);
        let eligible = filter_eligible(&reg, tmp.path(), "active");
        let ids: Vec<&str> = eligible.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn default_registry_path_reads_env_override() {
        let saved = std::env::var("FORGE_WORKSPACE_REGISTRY").ok();
        std::env::set_var("FORGE_WORKSPACE_REGISTRY", "/tmp/custom.json");
        let p = default_registry_path(None);
        assert_eq!(p, PathBuf::from("/tmp/custom.json"));
        match saved {
            Some(v) => std::env::set_var("FORGE_WORKSPACE_REGISTRY", v),
            None => std::env::remove_var("FORGE_WORKSPACE_REGISTRY"),
        }
    }
}