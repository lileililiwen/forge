//! `platform.identity-subject` payload.

use serde::{Deserialize, Serialize};

/// Identity subject document.
///
/// The wire shape mirrors `schemas/identity-subject.schema.json` in
/// the `platform-contracts` repository. The Rust type is permissive
/// about optional fields so producers can build documents without
/// jumping through `Option`; the JSON Schema is the source of truth
/// for what is actually accepted on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentitySubject {
    /// Stable identifier of the subject within the owning tenant.
    pub subject_id: String,
    /// Kind of subject the document describes.
    pub subject_type: SubjectType,
    /// Tenant under which the subject is acting.
    pub tenant_id: String,
    /// Optional human-readable label. MUST NOT contain secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Authentication context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Authentication>,
}

/// Subject kind. Mirrors the `subject_type` enum in the
/// `platform-contracts` schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Authentication block. `method` and `issued_at` are required when
/// the block is present; the platform schema declares them as
/// required inside the `authentication` object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authentication {
    /// Authentication method.
    pub method: AuthenticationMethod,
    /// RFC3339 timestamp the authentication was issued.
    pub issued_at: String,
    /// Optional RFC3339 expiration timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Authentication method enum. Mirrors the `authentication.method`
/// enum in the platform schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
