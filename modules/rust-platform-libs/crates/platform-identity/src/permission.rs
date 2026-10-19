//! Permission key, decision, and reason code.

use platform_core::bounded::BoundedString;
use serde::{Deserialize, Serialize};

/// Permission key.
///
/// The key is bounded at 128 characters to mirror the
/// `platform-contracts` schema's `maxLength: 128` on `action`.
pub type Permission = BoundedString<128>;

/// Authorization outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    /// Authorization granted.
    Allow,
    /// Authorization denied.
    Deny,
}

/// Reason code that complements a [`PermissionDecision`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionReason {
    /// A role matched.
    RoleMatch,
    /// A policy matched.
    PolicyMatch,
    /// An explicit deny rule matched.
    ExplicitDeny,
    /// A required role was missing.
    MissingRole,
    /// The authorization was expired.
    Expired,
    /// Other (catch-all).
    Other,
}
