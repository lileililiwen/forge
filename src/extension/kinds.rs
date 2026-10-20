//! The deploy target kind registry.
//!
//! The kinds live here so the manifest validator, the
//! configuration validator and the deployment plan read one list
//! and produce one wording. A kind that is known but not wired
//! into this build is refused as such, everywhere it appears.

use super::state::ExtensionState;

/// The adapter runs on this host.
pub const TARGET_LOCAL: &str = "local";

/// The adapter drives a Compose project.
pub const TARGET_DOCKER_COMPOSE: &str = "docker-compose";

/// The SSH host adapter is a later change.
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
}

/// The registry, in the order the surface renders it.
pub const DEPLOY_TARGET_KINDS: &[DeployTargetKind] = &[
    DeployTargetKind {
        kind: TARGET_LOCAL,
        wired: true,
        summary: "The adapter runs on this host.",
    },
    DeployTargetKind {
        kind: TARGET_DOCKER_COMPOSE,
        wired: true,
        summary: "The adapter drives a Compose project.",
    },
    DeployTargetKind {
        kind: TARGET_JENKINS,
        wired: true,
        summary: "The adapter drives a Jenkins job.",
    },
    DeployTargetKind {
        kind: TARGET_MAC_RUNTIME,
        wired: true,
        summary: "The adapter runs here and sends runtime operations to the target host.",
    },
    DeployTargetKind {
        kind: TARGET_SSH,
        wired: false,
        summary: "The SSH host adapter is a later change.",
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
