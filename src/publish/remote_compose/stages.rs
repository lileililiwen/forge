//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{Classification, CommandResult, CommandSpec};
use crate::core::ForgeError;
use crate::publish::db_overlay;
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

use super::constants::{COMPOSE_CANDIDATES, DOCKER_BIN, DOCKER_PATH, MAX_COMPOSE_BYTES};
use super::model::RemoteConfig;

/// The Compose project name a decoupled deploy owns. Distinct from
/// the retained legacy `jenkins-<project>` identity so a rollback is
/// visible in `docker ps` rather than silently reusing containers.
pub fn compose_project_identity(project_id: &str) -> String {
    format!("forge-{project_id}")
}

pub(super) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The Compose file a local project checkout ships, when present.
pub(super) fn local_compose_file(project_dir: &Path) -> Option<String> {
    COMPOSE_CANDIDATES
        .iter()
        .find(|candidate| project_dir.join(candidate).is_file())
        .map(|candidate| (*candidate).to_string())
}

/// Compose profiles declared by the local checkout's Compose file,
/// sorted and deduplicated. Best-effort: any read or parse failure
/// means no profiles (the previous behavior), never a refusal.
pub(super) fn compose_profiles(project_dir: &Path, compose_file: &str) -> Vec<String> {
    let path = project_dir.join(compose_file);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut profiles = BTreeSet::new();
    if let Some(services) = yaml.get("services").and_then(|v| v.as_mapping()) {
        for (_, service) in services {
            if let Some(list) = service.get("profiles").and_then(|v| v.as_sequence()) {
                for entry in list {
                    if let Some(name) = entry.as_str() {
                        let name = name.trim();
                        if !name.is_empty() {
                            profiles.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }
    profiles.into_iter().collect()
}

/// `--profile` argv for the given profiles, in compose flag position
/// (directly after `compose`, before `-f`).
pub(super) fn profile_args(profiles: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(profiles.len() * 2);
    for profile in profiles {
        argv.push("--profile".to_string());
        argv.push(profile.clone());
    }
    argv
}

/// Parse the local checkout's Compose file into the same JSON shape
/// `docker compose config --format json` produces on the target.
/// Used only when the target has no Compose file yet (first sync):
/// returns `Ok(None)` when the local file is absent so the caller
/// keeps its original target error, and refuses an unreadable or
/// unparseable local document instead of deploying blind.
pub(super) fn read_local_compose(
    project_dir: &Path,
    compose_file: &str,
) -> Result<Option<Value>, ForgeError> {
    let path = project_dir.join(compose_file);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot read local Compose file {}: {error}", path.display()),
    })?;
    if bytes.len() > MAX_COMPOSE_BYTES {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} is {} bytes, above the {MAX_COMPOSE_BYTES} byte bound",
                path.display(),
                bytes.len()
            ),
        });
    }
    let text = String::from_utf8(bytes).map_err(|_| ForgeError::PublishInvalid {
        reason: format!("local Compose file {} is not valid UTF-8", path.display()),
    })?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} is not valid YAML: {error}",
                path.display()
            ),
        })?;
    serde_json::to_value(yaml)
        .map(Some)
        .map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "local Compose file {} does not project to JSON: {error}",
                path.display()
            ),
        })
}

pub(super) fn validate_project_id(project_id: &str) -> Result<(), ForgeError> {
    if project_id.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if !project_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "project id `{project_id}` must be kebab/snake-case (letters, digits, dash, underscore)"
            ),
        });
    }
    Ok(())
}

/// `ssh <target> env PATH=… BUILDKIT_PROGRESS=plain <docker> …`. The
/// remote `env` keeps the command free of any shell script while
/// giving Docker Desktop's helper directory to non-interactive SSH
/// sessions and plain build output to BuildKit.
pub(super) fn docker_command(config: &RemoteConfig, label: &str) -> CommandSpec {
    let mut spec = CommandSpec::new("ssh", label);
    spec.args.push(OsString::from(config.ssh_target.as_str()));
    spec.args.push(OsString::from("env"));
    spec.args
        .push(OsString::from(format!("PATH={DOCKER_PATH}")));
    spec.args.push(OsString::from("BUILDKIT_PROGRESS=plain"));
    spec.args.push(OsString::from(DOCKER_BIN));
    spec
}

pub(super) fn classify_sync(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("project source tree synced to the target");
    }
    let recovery = match result.status {
        255 | 12 => vec![
            format!(
                "verify the SSH alias `{}` resolves in ~/.ssh/config",
                config.ssh_target
            ),
            "partial state may exist on the target; rerun `forge publish sync` to repair"
                .to_string(),
        ],
        _ => vec!["inspect the ssh/rsync output above for the underlying cause".to_string()],
    };
    Classification::failed("sync failed", recovery)
}

pub(super) fn classify_prepare(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("host ports allocated and runtime documents shipped");
    }
    Classification::failed(
        "publish prepare failed",
        vec![
            format!("verify {} is writable on the target", config.runtime_root),
            "verify the target is reachable and the port registry parses".to_string(),
        ],
    )
}

pub(super) fn classify_db(result: &CommandResult) -> Classification {
    if result.success() {
        return Classification::done("shared PostgreSQL is running on the target");
    }
    let recovery = match result.status {
        1 => vec![
            format!(
                "verify Docker is running on the target and {}/compose.yml is present",
                "shared-infra"
            ),
            format!(
                "create the network with `docker network create {}`",
                db_overlay::SHARED_DB_NETWORK
            ),
        ],
        _ => vec![
            "verify the shared-infrastructure Compose file and its admin env file".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("shared PostgreSQL is unavailable", recovery)
}

pub(super) fn classify_deploy(result: &CommandResult, config: &RemoteConfig) -> Classification {
    if result.success() {
        return Classification::done("compose project started on the target");
    }
    let recovery = match result.status {
        1 => vec![
            "run `forge publish prepare <project>` first to allocate ports and ship the runtime documents"
                .to_string(),
            format!(
                "verify {}/<project>/ports.compose.yml exists on the target",
                config.runtime_root
            ),
        ],
        _ => vec![
            "review the target compose output above for the failing service".to_string(),
            format!("review stderr above (exit {})", result.status),
        ],
    };
    Classification::failed("compose deploy rejected the request", recovery)
}

pub(super) fn classify_generic(result: &CommandResult) -> Classification {
    if result.success() {
        Classification::done("stage completed")
    } else {
        Classification::failed(
            "stage failed",
            vec![format!("inspect exit {}", result.status)],
        )
    }
}
