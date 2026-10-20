//! The deploy target kind registry.
//!
//! The kinds live here so the manifest validator, the
//! configuration validator and the deployment plan read one list
//! and produce one wording. A kind that is known but not wired
//! into this build is refused as such, everywhere it appears.
//!
//! A kind also says what it needs. The registry entry carries the
//! target fields a target of that kind must state, the variable
//! that names the kind's own adapter binary and the binary the
//! kind uses when nothing else names one. The requirement lives
//! next to the kind, so the next branch inherits it instead of
//! restating it in a validator.

use super::state::ExtensionState;

/// The adapter runs on this host.
pub const TARGET_LOCAL: &str = "local";

/// The adapter drives a Compose project.
pub const TARGET_DOCKER_COMPOSE: &str = "docker-compose";

/// The SSH host adapter copies the artifact to the named host.
pub const TARGET_SSH: &str = "ssh";

/// The adapter drives a Jenkins job.
pub const TARGET_JENKINS: &str = "jenkins";

/// The adapter runs here and sends runtime operations to the
/// target host.
pub const TARGET_MAC_RUNTIME: &str = "mac-runtime";

/// One deploy target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeployTargetKind {
    /// The stable kind name a manifest declares.
    pub kind: &'static str,
    /// False when the kind is known but not wired into this build.
    pub wired: bool,
    /// What the kind does.
    pub summary: &'static str,
    /// The target fields a target of this kind must state.
    pub required_fields: &'static [&'static str],
    /// The variable that names this kind's own adapter binary.
    pub adapter_env: Option<&'static str>,
    /// The adapter binary this kind uses when nothing else names one.
    pub default_adapter: &'static str,
}

/// The registry, in the order the surface renders it.
pub const DEPLOY_TARGET_KINDS: &[DeployTargetKind] = &[
    DeployTargetKind {
        kind: TARGET_LOCAL,
        wired: true,
        summary: "The adapter runs on this host.",
        required_fields: &[],
        adapter_env: None,
        default_adapter: "forge-deployer",
    },
    DeployTargetKind {
        kind: TARGET_DOCKER_COMPOSE,
        wired: true,
        summary: "The adapter drives a Compose project.",
        required_fields: &[],
        adapter_env: None,
        default_adapter: "forge-deployer",
    },
    DeployTargetKind {
        kind: TARGET_JENKINS,
        wired: true,
        summary: "The adapter drives a Jenkins job.",
        required_fields: &[],
        adapter_env: None,
        default_adapter: "forge-deployer",
    },
    DeployTargetKind {
        kind: TARGET_MAC_RUNTIME,
        wired: true,
        summary: "The adapter runs here and sends runtime operations to the target host.",
        required_fields: &[],
        adapter_env: None,
        default_adapter: "forge-deployer",
    },
    DeployTargetKind {
        kind: TARGET_SSH,
        wired: true,
        summary: "The adapter copies the artifact to the named host and observes it.",
        required_fields: &["host", "path"],
        adapter_env: Some("FORGE_DEPLOYER_SSH_BIN"),
        default_adapter: "forge-deployer-ssh",
    },
];

/// The wired kinds, rendered once. Every refusal that names the
/// supported kinds names this list, so the manifest, the
/// configuration and the plan cannot drift apart.
pub fn supported_target_kinds() -> String {
    let wired: Vec<String> = DEPLOY_TARGET_KINDS
        .iter()
        .filter(|entry| entry.wired)
        .map(|entry| format!("`{}`", entry.kind))
        .collect();
    match wired.len() {
        0 => String::new(),
        1 => wired[0].clone(),
        _ => {
            let last = wired[wired.len() - 1].clone();
            let head = wired[..wired.len() - 1].join(", ");
            format!("{head}, and {last}")
        }
    }
}

/// The target fields a kind requires. A target that does not
/// state them is refused by name, so the requirement never has to
/// be restated in a validator. An unknown kind requires nothing.
pub fn required_target_fields(kind: &str) -> &'static [&'static str] {
    DEPLOY_TARGET_KINDS
        .iter()
        .find(|entry| entry.kind == kind)
        .map(|entry| entry.required_fields)
        .unwrap_or(&[])
}

/// The adapter binary a kind uses when nothing else names one.
/// None for a kind the registry does not know.
pub fn default_adapter_for_kind(kind: &str) -> Option<&'static str> {
    DEPLOY_TARGET_KINDS
        .iter()
        .find(|entry| entry.kind == kind)
        .map(|entry| entry.default_adapter)
}

/// The state of one deploy target kind: `ready` when the kind is
/// wired into this build, `invalid` with the shared wording when
/// it is known but unwired, and `invalid` with the shared wording
/// when it is unknown.
pub fn deploy_target_kind_state(kind: &str) -> ExtensionState {
    match DEPLOY_TARGET_KINDS.iter().find(|entry| entry.kind == kind) {
        Some(entry) if entry.wired => ExtensionState::Ready,
        Some(_) => ExtensionState::Invalid {
            reason: format!(
                "deployment target kind `{kind}` is known but not yet wired; v0.1.0 supports {}",
                supported_target_kinds()
            ),
        },
        None => ExtensionState::Invalid {
            reason: format!(
                "deployment target kind `{kind}` is not supported; expected one of {}",
                supported_target_kinds()
            ),
        },
    }
}
