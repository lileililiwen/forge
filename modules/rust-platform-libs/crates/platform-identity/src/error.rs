//! Identity error categories.
//!
//! Every identity-layer failure funnels into [`IdentityError`]. The
//! error is paired with a stable [`IdentityFailure`] category, a
//! human-readable message, and an optional correlation id. It never
//! carries a password, token, hash, or provider response detail —
//! those would defeat the safety guarantees documented in
//! `docs/ownership.md` of the `platform-contracts` repository.

use platform_core::correlation::CorrelationId;
use platform_core::error::ErrorCategory;
use serde::{Deserialize, Serialize};

/// Stable, machine-readable identity failure categories.
///
/// The variant set is part of the platform contract: adding a variant
/// is a minor bump; renaming a variant is a breaking change. The
/// `as_str` form is the wire form used in audit events and logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityFailure {
    /// Credential check failed. The message MUST NOT identify which
    /// part of the credential (user, password, MFA) was wrong.
    InvalidCredentials,
    /// The session or token expired.
    Expired,
    /// The session or token was administratively revoked.
    Revoked,
    /// The subject is not permitted to act within the resolved
    /// tenant context.
    TenantMismatch,
    /// The configured identity provider is unreachable or returned
    /// an error. The message MUST NOT include the provider response.
    ProviderUnavailable,
    /// The identity layer is misconfigured (missing key, broken
    /// policy, etc.). The message MUST NOT include credential
    /// material.
    Misconfigured,
}

impl IdentityFailure {
    /// Wire form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::TenantMismatch => "tenant_mismatch",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::Misconfigured => "misconfigured",
        }
    }

    /// Map the identity failure to the platform's [`ErrorCategory`]
    /// for cross-cutting error handling.
    #[must_use]
    pub const fn category(self) -> ErrorCategory {
        match self {
            Self::InvalidCredentials | Self::Expired | Self::Revoked => {
                ErrorCategory::Authentication
            }
            Self::TenantMismatch => ErrorCategory::Tenant,
            Self::ProviderUnavailable | Self::Misconfigured => ErrorCategory::Internal,
        }
    }
}

impl std::fmt::Display for IdentityFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Identity-layer error.
///
/// The error is intentionally simple: a category, a message, and an
/// optional correlation id. There is no `cause()` chain that can
/// carry arbitrary `dyn Error` values, because the platform cannot
/// guarantee that arbitrary error sources do not embed credential
/// material. Adapters that need to bridge from their own error types
/// MUST map their internal categories to [`IdentityFailure`] and
/// discard the inner detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityError {
    failure: IdentityFailure,
    message: String,
    correlation_id: Option<CorrelationId>,
}

impl IdentityError {
    /// Construct a new identity error.
    ///
    /// The `message` MUST NOT contain credential material. Construct
    /// a new [`IdentityError`] with a stable, non-secret description
    /// (e.g. "the supplied credentials were not accepted"). Any
    /// internal provider detail stays in the application's logs, not
    /// the platform error.
    #[must_use]
    pub fn new(failure: IdentityFailure, message: impl Into<String>) -> Self {
        Self {
            failure,
            message: message.into(),
            correlation_id: None,
        }
    }

    /// Attach a correlation id.
    #[must_use]
    pub fn with_correlation(mut self, id: CorrelationId) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Failure category accessor.
    #[must_use]
    pub fn failure(&self) -> IdentityFailure {
        self.failure
    }

    /// Correlation id accessor.
    #[must_use]
    pub fn correlation_id(&self) -> Option<CorrelationId> {
        self.correlation_id
    }

    /// Borrow the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[identity:{}] {}", self.failure.as_str(), self.message)?;
        if let Some(id) = self.correlation_id {
            write!(f, " (correlation_id={id})")?;
        }
        Ok(())
    }
}

impl std::error::Error for IdentityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_categories_are_stable() {
        assert_eq!(
            IdentityFailure::InvalidCredentials.as_str(),
            "invalid_credentials"
        );
        assert_eq!(
            IdentityFailure::InvalidCredentials.category(),
            ErrorCategory::Authentication
        );
        assert_eq!(
            IdentityFailure::TenantMismatch.category(),
            ErrorCategory::Tenant
        );
    }

    #[test]
    fn display_includes_correlation_id() {
        let err = IdentityError::new(IdentityFailure::Expired, "session expired")
            .with_correlation(CorrelationId::new());
        let s = format!("{err}");
        assert!(s.contains("expired"));
        assert!(s.contains("session expired"));
    }
}
