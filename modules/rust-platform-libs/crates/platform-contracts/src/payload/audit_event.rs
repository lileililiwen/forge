//! `platform.audit-event` payload.

use serde::{Deserialize, Serialize};

use super::identity_subject::SubjectType;

/// Audit event document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Stable event identifier.
    pub event_id: String,
    /// RFC3339 timestamp the event occurred.
    pub occurred_at: String,
    /// Event category.
    pub category: AuditCategory,
    /// Operation being audited, e.g. `read:document`.
    pub action: String,
    /// Outcome of the audited action.
    pub outcome: AuditOutcome,
    /// Actor that triggered the event.
    pub actor: AuditActor,
    /// Tenant under which the event occurred.
    pub tenant_id: String,
    /// Optional resource identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Optional free-form context. MUST NOT contain raw credentials;
    /// the validator rejects any field whose name contains a secret
    /// substring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Audit event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    /// Authorization-related event.
    Permission,
    /// Data-access event.
    DataAccess,
    /// Configuration change.
    Configuration,
    /// Authentication event.
    Authentication,
    /// Lifecycle event.
    Lifecycle,
    /// Other.
    Other,
}

/// Audit event outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Operation succeeded.
    Success,
    /// Operation failed.
    Failure,
    /// Operation was denied.
    Denied,
    /// Outcome is unknown.
    Unknown,
}

/// Actor of an audit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditActor {
    /// Subject identifier of the actor.
    pub subject_id: String,
    /// Kind of actor.
    pub subject_type: SubjectType,
}
