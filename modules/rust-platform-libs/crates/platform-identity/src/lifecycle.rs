//! Lifecycle states shared by subjects, sessions, and tenants.

use serde::{Deserialize, Serialize};

/// Lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    /// Newly created, not yet activated.
    Created,
    /// Active and usable.
    Active,
    /// Temporarily disabled. Reversible.
    Suspended,
    /// Permanently disabled. Not reversible.
    Revoked,
}

impl Lifecycle {
    /// True if the lifecycle state allows operations to proceed.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_active_state_is_operational() {
        assert!(Lifecycle::Active.is_active());
        assert!(!Lifecycle::Created.is_active());
        assert!(!Lifecycle::Suspended.is_active());
        assert!(!Lifecycle::Revoked.is_active());
    }
}
