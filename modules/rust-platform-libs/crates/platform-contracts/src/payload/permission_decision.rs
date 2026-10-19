//! `platform.permission-decision` payload.

use serde::{Deserialize, Serialize};

/// Permission decision document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDecision {
    /// Stable decision identifier.
    pub decision_id: String,
    /// Subject the decision applies to.
    pub subject_id: String,
    /// Operation being authorized, e.g. `read:document`.
    pub action: String,
    /// Resource identifier the decision applies to, e.g. `doc:12345`.
    pub resource: String,
    /// Tenant under which the decision is made.
    pub tenant_id: String,
    /// Authorization outcome.
    pub decision: PermissionOutcome,
    /// Machine-readable reason code.
    pub reason_code: PermissionReason,
    /// RFC3339 timestamp the decision was evaluated.
    pub evaluated_at: String,
    /// Optional policy identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
}

/// Outcome of a permission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOutcome {
    /// Authorization granted.
    Allow,
    /// Authorization denied.
    Deny,
}

/// Reason code that complements the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
