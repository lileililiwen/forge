//! The default `release:` and `deployment:` blocks of a manifest Forge writes.
//!
//! One owner for both blocks: the generation writer
//! (`src/generate/request.rs`) and the import writer
//! (`src/import/detect.rs`) render them from this module, so the two
//! writers cannot drift apart. The blocks are rendered as YAML text
//! because the manifest contract types are `Deserialize`-only; a test
//! parses the rendered result back through the real parser and resolves
//! it through the release and deploy contracts, so the rendered shape
//! cannot drift from the contract either.
//!
//! The defaults are derived, never fabricated: the release block names
//! the checks a project satisfies with its own source, and the
//! deployment block declares an artifact only when the project has one.

/// The check kinds a fresh project declares.
///
/// `doctor` is Forge's own assessment of the project and `test` runs the
/// profile's own test command: the two checks a project can satisfy with
/// its own source. `gate` and `driftwatch` are not declared by default
/// because they need evidence the owner produces by running them, and a
/// declared check that cannot run blocks the release.
pub const DEFAULT_RELEASE_CHECKS: [&str; 2] = ["doctor", "test"];

/// The Compose file names, in precedence order.
///
/// A Compose file names the services to run, so it is the more specific
/// artifact and wins over a Dockerfile.
pub const COMPOSE_FILES: [&str; 2] = ["docker-compose.yml", "compose.yaml"];

/// The container file a project ships on its own.
pub const DOCKERFILE: &str = "Dockerfile";

/// The versioning scheme a fresh project declares. `semver` is the only
/// scheme the release contract accepts.
const VERSIONING: &str = "semver";

/// The name of the deployment target Forge declares. Kebab-case and
/// valid under the deploy contract.
const TARGET_NAME: &str = "local";

/// The adapter kind for a project that ships its own container file.
const KIND_LOCAL: &str = "local";

/// The adapter kind for a project that ships a Compose file.
const KIND_DOCKER_COMPOSE: &str = "docker-compose";

/// The default deployment kind and artifact of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentDefaults {
    /// The adapter kind of the declared target.
    pub kind: &'static str,
    /// The artifact the declared target consumes, when the project has
    /// one. A declared artifact that is missing is refused at deploy
    /// time, so it is declared only when the file exists.
    pub artifact: Option<String>,
}

/// The default deployment kind and artifact for the files a project has.
///
/// `present` names the files that exist; only the container file names
/// are considered. A Compose file gives a `docker-compose` target and
/// that file as the artifact. A Dockerfile alone gives a `local` target
/// with that file as the artifact. A project with neither gets a
/// `local` target and no artifact.
pub fn deployment_defaults(present: &[&str]) -> DeploymentDefaults {
    for compose in COMPOSE_FILES {
        if present.contains(&compose) {
            return DeploymentDefaults {
                kind: KIND_DOCKER_COMPOSE,
                artifact: Some(compose.to_string()),
            };
        }
    }
    if present.contains(&DOCKERFILE) {
        return DeploymentDefaults {
            kind: KIND_LOCAL,
            artifact: Some(DOCKERFILE.to_string()),
        };
    }
    DeploymentDefaults {
        kind: KIND_LOCAL,
        artifact: None,
    }
}

/// Render the `release:` block for the given check kinds.
///
/// Every line ends with a newline, so the block can be appended to a
/// manifest that is built line by line.
pub fn release_block(checks: &[&str]) -> String {
    let mut text = String::from("release:\n");
    text.push_str(&format!("  versioning: {VERSIONING}\n"));
    if !checks.is_empty() {
        text.push_str("  checks:\n");
        for check in checks {
            text.push_str(&format!("    - kind: {check}\n"));
        }
    }
    text
}

/// Render the `deployment:` block for one target.
///
/// The block never declares the legacy `type`/`target` fields: the deploy
/// contract refuses a manifest that mixes the two forms. It declares no
/// health check either, because the project runs no service yet.
pub fn deployment_block(kind: &str, artifact: Option<&str>) -> String {
    let mut text = String::from("deployment:\n");
    if let Some(artifact) = artifact.filter(|path| !path.trim().is_empty()) {
        text.push_str(&format!("  artifact: {artifact}\n"));
    }
    text.push_str("  targets:\n");
    text.push_str(&format!("    - name: {TARGET_NAME}\n"));
    text.push_str(&format!("      kind: {kind}\n"));
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::Manifest;
    use std::path::Path;

    #[test]
    fn a_compose_file_wins_over_a_dockerfile() {
        let defaults = deployment_defaults(&["Dockerfile", "docker-compose.yml"]);
        assert_eq!(defaults.kind, KIND_DOCKER_COMPOSE);
        assert_eq!(defaults.artifact.as_deref(), Some("docker-compose.yml"));
    }

    #[test]
    fn the_alternate_compose_name_is_recognised() {
        let defaults = deployment_defaults(&["compose.yaml"]);
        assert_eq!(defaults.kind, KIND_DOCKER_COMPOSE);
        assert_eq!(defaults.artifact.as_deref(), Some("compose.yaml"));
    }

    #[test]
    fn a_dockerfile_alone_gives_a_local_target() {
        let defaults = deployment_defaults(&["Dockerfile"]);
        assert_eq!(defaults.kind, KIND_LOCAL);
        assert_eq!(defaults.artifact.as_deref(), Some("Dockerfile"));
    }

    #[test]
    fn a_project_without_a_container_file_declares_no_artifact() {
        let defaults = deployment_defaults(&["Cargo.toml", "src/main.rs"]);
        assert_eq!(defaults.kind, KIND_LOCAL);
        assert_eq!(defaults.artifact, None);
    }

    #[test]
    fn the_release_block_names_the_default_checks() {
        assert_eq!(
            release_block(&DEFAULT_RELEASE_CHECKS),
            "release:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n"
        );
    }

    #[test]
    fn the_deployment_block_omits_an_absent_artifact() {
        assert_eq!(
            deployment_block(KIND_LOCAL, None),
            "deployment:\n  targets:\n    - name: local\n      kind: local\n"
        );
    }

    #[test]
    fn the_deployment_block_never_declares_the_legacy_fields() {
        let block = deployment_block(KIND_DOCKER_COMPOSE, Some("docker-compose.yml"));
        assert!(!block.contains("type:"), "{block}");
        assert!(!block.contains("target:"), "{block}");
        assert!(!block.contains("health"), "{block}");
    }

    #[test]
    fn the_rendered_blocks_resolve_through_both_contracts() {
        let text = format!(
            "schema: 1\nproject:\n  id: demo-app\n  name: demo-app\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\n{}{}",
            release_block(&DEFAULT_RELEASE_CHECKS),
            deployment_block(KIND_LOCAL, Some(DOCKERFILE)),
        );
        let manifest = Manifest::parse(Path::new("forge.yaml"), text.as_bytes())
            .expect("the rendered manifest must parse");
        let release = crate::release::prepare::release_config_from_manifest(&manifest)
            .expect("the release configuration must resolve");
        assert_eq!(release.versioning, "semver");
        assert_eq!(
            release.checks,
            vec![
                crate::release::model::CheckSpec {
                    kind: "doctor".to_string()
                },
                crate::release::model::CheckSpec {
                    kind: "test".to_string()
                },
            ]
        );
        let deploy = crate::deploy::prepare::deploy_config_from_manifest(&manifest)
            .expect("the deployment configuration must resolve");
        assert_eq!(deploy.default_target, "local");
        assert_eq!(deploy.adapter, KIND_LOCAL);
        assert_eq!(deploy.artifact.as_deref(), Some("Dockerfile"));
    }
}
