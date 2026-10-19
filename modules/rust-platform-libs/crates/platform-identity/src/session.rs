//! Session.
//!
//! A [`Session`] is the platform's typed view of an authenticated
//! session. The application owns the storage and the transport; the
//! platform only sees the resolved struct.

use platform_core::correlation::CorrelationId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::lifecycle::Lifecycle;
use crate::subject::Subject;

/// A resolved session. The application hydrates this from its store
/// when an authenticated request arrives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Session identifier. Opaque; not the cookie or token value.
    id: String,
    /// Resolved subject.
    subject: Subject,
    /// Lifecycle state.
    lifecycle: Lifecycle,
    /// RFC3339 timestamp the session was issued.
    #[serde(with = "time::serde::rfc3339")]
    issued_at: OffsetDateTime,
    /// RFC3339 timestamp the session expires.
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
    /// Correlation id captured at session creation.
    correlation_id: CorrelationId,
}

impl Session {
    /// Construct a new session.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        subject: Subject,
        issued_at: OffsetDateTime,
        expires_at: OffsetDateTime,
        correlation_id: CorrelationId,
    ) -> Self {
        Self {
            id: id.into(),
            subject,
            lifecycle: Lifecycle::Active,
            issued_at,
            expires_at,
            correlation_id,
        }
    }

    /// Construct a session with an explicit lifecycle.
    #[must_use]
    pub fn with_lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Session identifier accessor.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Subject accessor.
    #[must_use]
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// Lifecycle accessor.
    #[must_use]
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }

    /// Issued-at accessor.
    #[must_use]
    pub fn issued_at(&self) -> OffsetDateTime {
        self.issued_at
    }

    /// Expires-at accessor.
    #[must_use]
    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }

    /// Correlation id accessor.
    #[must_use]
    pub fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }

    /// Returns true iff the session is past its `expires_at`.
    #[must_use]
    pub fn is_expired(&self, now: OffsetDateTime) -> bool {
        now >= self.expires_at
    }
}
