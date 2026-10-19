//! Safe error model.
//!
//! [`SafeError`] is the error type used by every platform crate. It
//! pairs a stable [`ErrorCategory`] with a human-readable message
//! and an opaque, structured cause chain. The error never carries
//! credential material; secrets belong in [`crate::redaction::Secret`]
//! and must not be attached to errors.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::correlation::CorrelationId;

/// Stable, machine-readable error category.
///
/// Categories are part of the platform contract: adding a new variant
/// is a minor bump; renaming a variant is a breaking change. The
/// `as_str` form is what shows up in log lines and the JSON wire
/// format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Authentication failed or no subject is present.
    Authentication,
    /// Authorization was denied.
    Permission,
    /// Tenant boundary or isolation mismatch.
    Tenant,
    /// Caller-supplied input failed validation.
    Validation,
    /// The requested resource does not exist.
    NotFound,
    /// The request collides with existing state.
    Conflict,
    /// A downstream dependency is unavailable.
    Unavailable,
    /// Contract envelope, schema, or version mismatch.
    Compatibility,
    /// Catch-all for unexpected internal failures.
    Internal,
}

impl ErrorCategory {
    /// Stable wire form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Permission => "permission",
            Self::Tenant => "tenant",
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Unavailable => "unavailable",
            Self::Compatibility => "compatibility",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Distinguishes errors raised by the platform itself from errors
/// carried through from an application-owned adapter. The category is
/// the same in both cases; the kind is documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeErrorKind {
    /// Raised inside a platform crate.
    Platform,
    /// Raised by an application-owned adapter that opted to produce
    /// a `SafeError` rather than its own error type.
    Adapter,
}

/// The platform's safe error type.
///
/// `SafeError` carries:
///
/// * a stable [`ErrorCategory`],
/// * a [`SafeErrorKind`] that distinguishes platform-internal errors
///   from adapter-originated errors,
/// * a non-secret, human-readable message,
/// * an optional correlation id for log aggregation,
/// * a structured cause chain (each `cause` is itself a `SafeError`).
///
/// `SafeError` deliberately does **not** implement `std::error::Error`'s
/// `source()` to chain arbitrary error types; the platform cannot
/// guarantee that an arbitrary `dyn Error` does not contain
/// credential material. Application adapters that need to bridge to
/// their own error types should map their internal categories to
/// [`ErrorCategory`] explicitly and lose the inner detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeError {
    category: ErrorCategory,
    kind: SafeErrorKind,
    message: String,
    correlation_id: Option<CorrelationId>,
    cause: Option<Box<SafeError>>,
}

impl SafeError {
    /// Construct a new platform-originated error.
    #[must_use]
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            kind: SafeErrorKind::Platform,
            message: message.into(),
            correlation_id: None,
            cause: None,
        }
    }

    /// Construct an adapter-originated error.
    #[must_use]
    pub fn adapter(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            kind: SafeErrorKind::Adapter,
            message: message.into(),
            correlation_id: None,
            cause: None,
        }
    }

    /// Attach a correlation id.
    #[must_use]
    pub fn with_correlation(mut self, id: CorrelationId) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Attach a cause.
    #[must_use]
    pub fn with_cause(mut self, cause: SafeError) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Category accessor.
    #[must_use]
    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    /// Kind accessor.
    #[must_use]
    pub fn kind(&self) -> SafeErrorKind {
        self.kind
    }

    /// Correlation id accessor.
    #[must_use]
    pub fn correlation_id(&self) -> Option<CorrelationId> {
        self.correlation_id
    }

    /// Borrow the cause chain.
    #[must_use]
    pub fn cause_ref(&self) -> Option<&SafeError> {
        self.cause.as_deref()
    }
}

impl fmt::Display for SafeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category, self.message)?;
        if let Some(id) = self.correlation_id {
            write!(f, " (correlation_id={id})")?;
        }
        if let Some(cause) = &self.cause {
            write!(f, ": {cause}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SafeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_renders_stable_wire_form() {
        assert_eq!(ErrorCategory::Authentication.as_str(), "authentication");
        assert_eq!(ErrorCategory::Compatibility.as_str(), "compatibility");
    }

    #[test]
    fn display_includes_correlation_and_cause() {
        let root = SafeError::new(ErrorCategory::Unavailable, "upstream timed out");
        let id = CorrelationId::new();
        let outer = SafeError::new(ErrorCategory::Authentication, "could not authenticate")
            .with_correlation(id)
            .with_cause(root);
        let s = format!("{outer}");
        assert!(s.contains("authentication"));
        assert!(s.contains("could not authenticate"));
        assert!(s.contains("upstream timed out"));
    }

    #[test]
    fn category_is_serialized_in_snake_case() {
        let err = SafeError::new(ErrorCategory::Permission, "denied");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["category"], "permission");
        assert_eq!(json["kind"], "platform");
    }
}
