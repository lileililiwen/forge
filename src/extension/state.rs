//! The one extension state vocabulary.
//!
//! `ready`, `disabled`, `unavailable` and `invalid` are the four
//! states every extension boundary reports, and every state that
//! is not `ready` carries a reason that names the extension and
//! the reason.

use super::version::ContractVersion;

/// The state of one extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionState {
    /// Enabled, well-formed, compatible and executable here.
    Ready,
    /// Switched off by the operator.
    Disabled { reason: String },
    /// The declared command is not executable on this host.
    Unavailable { reason: String },
    /// The declaration is malformed, or names a version that does
    /// not satisfy the requirement.
    Invalid { reason: String },
}

impl ExtensionState {
    /// The stable label the transports render.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtensionState::Ready => "ready",
            ExtensionState::Disabled { .. } => "disabled",
            ExtensionState::Unavailable { .. } => "unavailable",
            ExtensionState::Invalid { .. } => "invalid",
        }
    }

    /// The reason a state is not `ready`. `None` for `ready`.
    pub fn reason(&self) -> Option<&str> {
        match self {
            ExtensionState::Ready => None,
            ExtensionState::Disabled { reason }
            | ExtensionState::Unavailable { reason }
            | ExtensionState::Invalid { reason } => Some(reason),
        }
    }

    /// Whether the extension may be used.
    pub fn is_ready(&self) -> bool {
        matches!(self, ExtensionState::Ready)
    }
}

/// The shape every extension's declaration takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    /// The extension's name. Every refusal names it.
    pub id: String,
    /// The contract the extension declares.
    pub contract: ContractVersion,
    /// Whether the operator switched the extension on.
    pub enabled: bool,
}

impl ExtensionDescriptor {
    /// Judge the declaration against the requirement, in one
    /// place: a switched-off extension is `disabled`, an
    /// extension that cannot run here is `unavailable`, and an
    /// incompatible declaration is `invalid` with the mismatch
    /// spelled out.
    pub fn state(&self, required: &ContractVersion, executable: bool) -> ExtensionState {
        if !self.enabled {
            return ExtensionState::Disabled {
                reason: format!("{} is disabled", self.id),
            };
        }
        if !executable {
            return ExtensionState::Unavailable {
                reason: format!("{} is not executable on this host", self.id),
            };
        }
        match self.contract.incompatibility_detail(required) {
            Some(detail) => ExtensionState::Invalid {
                reason: format!(
                    "{} declares the contract `{}`, which is incompatible with the required `{}`: {}",
                    self.id, self.contract, required, detail
                ),
            },
            None => ExtensionState::Ready,
        }
    }
}
