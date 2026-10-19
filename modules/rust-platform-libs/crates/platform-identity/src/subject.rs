//! Subject — the resolved identity of the current actor.

use platform_core::bounded::BoundedString;
use platform_core::id::Id;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::lifecycle::Lifecycle;

/// Subject identifier, bounded to 128 characters.
pub type SubjectId = Id<SubjectTag>;

/// Phantom tag for [`SubjectId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubjectTag {}

/// Authentication method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationMethod {
    /// Password-based authentication.
    Password,
    /// OAuth 2.0.
    OAuth2,
    /// Single sign-on.
    Sso,
    /// API key.
    ApiKey,
    /// Certificate.
    Certificate,
}

/// Authentication context. `method` and `issued_at` are required
/// when the block is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authentication {
    /// How the subject was authenticated.
    pub method: AuthenticationMethod,
    /// RFC3339 issued-at timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub issued_at: OffsetDateTime,
    /// Optional RFC3339 expires-at timestamp.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub expires_at: Option<OffsetDateTime>,
}

/// Subject kind. Mirrors the `subject_type` enum in the
/// `platform-contracts` schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    /// Interactive user.
    User,
    /// Non-interactive service.
    Service,
    /// API key holder.
    ApiKey,
    /// Unauthenticated request.
    Anonymous,
}

/// The resolved subject for the current request. The application
/// builds this from its store; the foundation only sees the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    /// Stable subject identifier.
    id: SubjectId,
    /// Kind of subject.
    subject_type: SubjectType,
    /// Tenant under which the subject is acting.
    tenant_id: Id<crate::tenant::TenantTag>,
    /// Optional human-readable display name.
    display_name: Option<BoundedString<256>>,
    /// Authentication context.
    authentication: Option<Authentication>,
    /// Lifecycle state.
    lifecycle: Lifecycle,
}

impl Subject {
    /// Construct a minimal subject.
    #[must_use]
    pub fn new(
        id: SubjectId,
        subject_type: SubjectType,
        tenant_id: Id<crate::tenant::TenantTag>,
    ) -> Self {
        Self {
            id,
            subject_type,
            tenant_id,
            display_name: None,
            authentication: None,
            lifecycle: Lifecycle::Active,
        }
    }

    /// Attach a display name.
    #[must_use]
    pub fn with_display_name(mut self, name: BoundedString<256>) -> Self {
        self.display_name = Some(name);
        self
    }

    /// Attach authentication context.
    #[must_use]
    pub fn with_authentication(mut self, auth: Authentication) -> Self {
        self.authentication = Some(auth);
        self
    }

    /// Attach a lifecycle state.
    #[must_use]
    pub fn with_lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Subject identifier accessor.
    #[must_use]
    pub fn id(&self) -> &SubjectId {
        &self.id
    }

    /// Subject type accessor.
    #[must_use]
    pub fn subject_type(&self) -> SubjectType {
        self.subject_type
    }

    /// Tenant identifier accessor.
    #[must_use]
    pub fn tenant_id(&self) -> &Id<crate::tenant::TenantTag> {
        &self.tenant_id
    }

    /// Display name accessor.
    #[must_use]
    pub fn display_name(&self) -> Option<&BoundedString<256>> {
        self.display_name.as_ref()
    }

    /// Authentication accessor.
    #[must_use]
    pub fn authentication(&self) -> Option<&Authentication> {
        self.authentication.as_ref()
    }

    /// Lifecycle accessor.
    #[must_use]
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }

    /// Convert into a serializable snapshot.
    #[must_use]
    pub fn snapshot(&self) -> SubjectSnapshot {
        SubjectSnapshot {
            subject_id: self.id.as_str().to_string(),
            subject_type: self.subject_type,
            tenant_id: self.tenant_id.as_str().to_string(),
            display_name: self.display_name.as_ref().map(|d| d.as_str().to_string()),
            authentication: self.authentication.clone(),
        }
    }
}

/// Serializable snapshot of a [`Subject`].
///
/// Identical wire shape to the `platform.identity-subject` contract
/// payload. Application code can serialize a snapshot to emit
/// contract documents without needing to import the
/// `platform-contracts` crate directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSnapshot {
    /// Stable subject identifier.
    pub subject_id: String,
    /// Subject kind.
    pub subject_type: SubjectType,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Optional human-readable display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional authentication context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Authentication>,
}
