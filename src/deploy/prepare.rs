//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::contract::{
    DEPLOY_STATE_DIR, HEALTH_DOCKER, HEALTH_HTTP, HEALTH_PROCESS, TARGET_DOCKER_COMPOSE,
    TARGET_JENKINS, TARGET_LOCAL, TARGET_MAC_RUNTIME, TARGET_SSH,
};
use super::model::{
    CapturedArtifact, DeployConfig, DeployHealthSpec, DeployIdentity, DeployReport, DeployState,
};

/// Hex SHA-256 over bytes. Reused for artifact identity.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn validate_target_kind(kind: &str) -> Result<(), ForgeError> {
    match kind {
        TARGET_LOCAL
        | TARGET_DOCKER_COMPOSE
        | TARGET_SSH
        | TARGET_JENKINS
        | TARGET_MAC_RUNTIME => Ok(()),
        other => Err(ForgeError::DeployInvalid {
                reason: format!(
                "deployment target kind `{other}` is not supported; expected one of `{TARGET_LOCAL}`, `{TARGET_DOCKER_COMPOSE}`, `{TARGET_JENKINS}`, `{TARGET_MAC_RUNTIME}`"
            ),
        }),
    }
}

pub(super) fn validate_target_name(name: &str) -> Result<(), ForgeError> {
    if name.is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "deployment target name must not be empty".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "deployment target name `{name}` must be kebab-case (lowercase letters, digits, single dashes)"
            ),
        });
    }
    Ok(())
}

/// Build a [`DeployConfig`] from a manifest's deployment
/// section. Convenience for callers that already have the
/// manifest.
pub fn deploy_config_from_manifest(manifest: &Manifest) -> Result<DeployConfig, ForgeError> {
    match manifest.deployment.as_ref() {
        Some(meta) => DeployConfig::from_manifest_meta(meta),
        None => Err(ForgeError::DeployInvalid {
            reason: format!(
                "project `{}` has no `deployment` section; declare one with at least one `deployment.targets[]` entry",
                manifest.project.id
            ),
        }),
    }
}

/// Compute the on-disk state path for one deploy. The path
/// is lexically scoped to the project directory, the project
/// id and the deploy identity so two projects (or two
/// deploys on the same project) can never share a state
/// file.
pub fn state_path_for(
    project_dir: &Path,
    project_id: &str,
    identity: &DeployIdentity,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "project id is required to resolve the deploy state path".to_string(),
        });
    }
    if identity.id.trim().is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "deploy id is required to resolve the deploy state path".to_string(),
        });
    }
    Ok(project_dir
        .join(DEPLOY_STATE_DIR)
        .join(project_id)
        .join(&identity.id)
        .join("state.json"))
}

pub fn load_deploy_state(path: &Path) -> Result<DeployState, ForgeError> {
    if !path.exists() {
        return Ok(DeployState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot read deploy state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(DeployState::default());
    }
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::DeployInvalid {
        reason: format!(
            "deploy state at {} is not valid JSON: {err}",
            path.display()
        ),
    })
}

pub fn save_deploy_state(path: &Path, state: &DeployState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::DeployInvalid {
            reason: format!(
                "cannot create deploy state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot serialize deploy state: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot write deploy state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot rename deploy state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Redact credential-like substrings from a piece of evidence.
/// The redaction is delegated to
/// [`crate::policy::redact_credentials`] so the deploy,
/// policy and distribution contracts share one definition
/// of "secret".
pub fn redact_deploy_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Capture the current working-tree revision. A non-git
/// project is refused before any deploy side effect runs.
pub fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|err| ForgeError::DeployInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "directory `{}` is not a git working tree with a HEAD commit: {}",
                dir.display(),
                stderr.trim()
            ),
        });
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(ForgeError::DeployInvalid {
            reason: "git rev-parse returned an empty SHA".to_string(),
        });
    }
    Ok(sha)
}

/// Load an artifact (relative to the project) and return
/// its content hash plus size. The full content is not
/// embedded in the report so the report stays small even
/// for large compose files.
pub fn load_artifact(project_dir: &Path, relative: &str) -> Result<CapturedArtifact, ForgeError> {
    if !lexically_inside(".", relative) {
        return Err(ForgeError::DeployInvalid {
            reason: format!("artifact `{relative}` resolves outside the project"),
        });
    }
    let path = project_dir.join(relative);
    if !path.is_file() {
        return Err(ForgeError::DeployInvalid {
            reason: format!(
                "artifact file `{}` does not exist; deploy requires a present artifact",
                path.display()
            ),
        });
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::DeployInvalid {
        reason: format!("cannot read artifact {}: {err}", path.display()),
    })?;
    let content_hash = hash_bytes(&bytes);
    Ok(CapturedArtifact {
        path: relative.to_string(),
        content_hash,
        byte_size: bytes.len() as u64,
    })
}

/// Render a [`DeployReport`] for human output. The transport
/// renders the same data the JSON envelope carries so a
/// partial run is observable on stdout.
pub fn render_report_human(report: &DeployReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!("deploy_id: {}", report.identity.id));
    lines.push(format!(
        "target: {} ({})",
        report.target.name, report.target.kind
    ));
    lines.push(format!(
        "source_revision: {}",
        report.identity.source_revision
    ));
    if let Some(artifact) = &report.artifact {
        lines.push(format!("artifact: {}", artifact.path));
        lines.push(format!("artifact_hash: {}", artifact.content_hash));
    }
    if let Some(health) = &report.health {
        lines.push(format!(
            "health: {} ({})",
            health.kind,
            health_label(health)
        ));
    }
    lines.push(format!("adapter: {}", report.adapter));
    lines.push(format!(
        "mode: {}",
        if report.dry_run { "dry-run" } else { "apply" }
    ));
    lines.push(format!("state: {}", report.state_path));
    if !report.stages.is_empty() {
        lines.push("stages:".to_string());
        for outcome in &report.stages {
            lines.push(format!(
                "  - {stage} target=`{target}` {status}: {note}",
                stage = outcome.stage,
                target = outcome.target,
                status = outcome.status,
                note = outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    if let Some(obs) = &report.observation {
        lines.push(format!(
            "observation: {} at {}: {}",
            obs.status, obs.observed_at, obs.detail
        ));
        for line in &obs.evidence {
            lines.push(format!("      evidence: {line}"));
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

fn health_label(health: &DeployHealthSpec) -> String {
    match health.kind.as_str() {
        HEALTH_DOCKER => format!("service={}", health.service.as_deref().unwrap_or("?")),
        HEALTH_HTTP => format!("url={}", health.url.as_deref().unwrap_or("?")),
        HEALTH_PROCESS => format!("process={}", health.process.as_deref().unwrap_or("?")),
        other => other.to_string(),
    }
}

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem. Absolute paths must already sit inside
/// the project, and `..` segments must never escape the
/// root. Symlinks are rechecked canonically at write time.
pub(super) fn lexically_inside(_project_dir: &str, rel: &str) -> bool {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return false;
    }
    let mut depth = 0i32;
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
        }
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::core::manifest::DeploymentMeta;
    use std::fs;
    use tempfile::TempDir;

    fn meta_with(targets: &str) -> DeploymentMeta {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n{targets}"
            )
            .as_bytes(),
        )
        .expect("manifest");
        manifest.deployment.expect("deployment section")
    }

    #[test]
    fn deploy_config_parses_single_typed_target() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: local\n");
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        assert_eq!(cfg.default_target, "home");
        assert_eq!(cfg.adapter, "local");
        assert_eq!(cfg.targets.len(), 1);
    }

    #[test]
    fn deploy_config_resolves_default_for_multiple_targets() {
        let meta = meta_with(
            "  default: vps\n  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: docker-compose\n",
        );
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        assert_eq!(cfg.default_target, "vps");
        assert_eq!(cfg.adapter, "mixed");
        assert_eq!(cfg.targets.len(), 2);
    }

    #[test]
    fn deploy_config_refuses_multiple_targets_without_default() {
        let meta = meta_with(
            "  targets:\n    - name: home\n      kind: local\n    - name: vps\n      kind: local\n",
        );
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_unknown_target_kind() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: bogus\n");
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_duplicate_target_name() {
        let meta = meta_with(
            "  default: home\n  targets:\n    - name: home\n      kind: local\n    - name: home\n      kind: docker-compose\n",
        );
        let err = DeployConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_artifact_outside_project() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: ../etc/passwd\n  targets:\n    - name: home\n      kind: local\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_accepts_linux_controlled_mac_runtime_target() {
        let meta = meta_with("  targets:\n    - name: mac-production\n      kind: mac-runtime\n");
        let config = DeployConfig::from_manifest_meta(&meta).expect("mac runtime target");
        assert_eq!(config.targets[0].kind, "mac-runtime");
    }

    #[test]
    fn deploy_config_accepts_legacy_single_target() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  type: docker-compose\n  target: home\n",
        )
        .expect("manifest");
        let cfg = DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap();
        assert_eq!(cfg.default_target, "home");
        assert_eq!(cfg.adapter, "docker-compose");
        assert_eq!(cfg.targets.len(), 1);
    }

    #[test]
    fn deploy_config_refuses_mixing_legacy_and_typed() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  type: docker-compose\n  target: home\n  targets:\n    - name: vps\n      kind: local\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_config_refuses_empty_block() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment: {}\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn deploy_identity_is_stable_for_same_inputs() {
        let a = DeployIdentity::derive("app", "home", "deadbeefcafe");
        let b = DeployIdentity::derive("app", "home", "deadbeefcafe");
        assert_eq!(a, b);
        let c = DeployIdentity::derive("app", "vps", "deadbeefcafe");
        assert_ne!(a, c);
        let d = DeployIdentity::derive("app", "home", "different");
        assert_ne!(a, d);
    }

    #[test]
    fn load_artifact_reads_present_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("docker-compose.yml"),
            "services:\n  app:\n    image: app\n",
        )
        .unwrap();
        let captured = load_artifact(tmp.path(), "docker-compose.yml").unwrap();
        assert_eq!(captured.path, "docker-compose.yml");
        assert!(captured.content_hash.len() == 64);
        assert!(captured.byte_size > 0);
    }

    #[test]
    fn load_artifact_refuses_missing_file() {
        let tmp = TempDir::new().unwrap();
        let err = load_artifact(tmp.path(), "docker-compose.yml").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn load_artifact_refuses_outside_project_path() {
        let err = load_artifact(Path::new("."), "../etc/passwd").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn target_lookup_rejects_unknown_name() {
        let meta = meta_with("  targets:\n    - name: home\n      kind: local\n");
        let cfg = DeployConfig::from_manifest_meta(&meta).unwrap();
        let err = cfg.target("vps").unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn health_spec_requires_kind() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: home\n      kind: local\n  health:\n    service: app\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }

    #[test]
    fn health_spec_rejects_unknown_kind() {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            b"schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\ndeployment:\n  artifact: docker-compose.yml\n  targets:\n    - name: home\n      kind: local\n  health:\n    kind: bogus\n",
        )
        .expect("manifest");
        let err =
            DeployConfig::from_manifest_meta(manifest.deployment.as_ref().unwrap()).unwrap_err();
        assert_eq!(err.code(), "deploy-invalid");
    }
}
