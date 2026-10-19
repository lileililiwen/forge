//! Audit sink trait and a no-op implementation for tests.

use std::sync::Mutex;

use platform_contracts::payload::audit_event::AuditEvent;

use crate::context::RedactedContextError;

/// Error returned by an [`AuditSink`].
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    /// The event was malformed (e.g. a context key matched a secret
    /// substring). The error never includes the offending value.
    #[error("audit event was rejected: {0}")]
    Rejected(String),
    /// The underlying storage failed. The error never includes
    /// event payload data.
    #[error("audit sink storage failure: {0}")]
    Storage(String),
    /// Conversion into the redaction context failed.
    #[error("redaction context error: {0}")]
    Redaction(#[from] RedactedContextError),
}

/// A sink that accepts audit events.
///
/// The application implements this trait against its storage of
/// choice. The platform guarantees that events reaching the sink have
/// already been redacted at the type level, but a defensive sink
/// SHOULD still re-check field names against the live
/// `platform-contracts` secret substrings before persisting.
pub trait AuditSink: Send + Sync {
    /// Record a single event. Returns `Ok(())` on successful accept.
    /// The sink MUST NOT panic; it MUST return an error rather than
    /// silently dropping an event.
    ///
    /// # Errors
    ///
    /// Returns [`SinkError::Rejected`] if the event fails the
    /// sink's own validation, or [`SinkError::Storage`] if the
    /// underlying storage is unavailable.
    fn record(&self, event: &AuditEvent) -> Result<(), SinkError>;
}

/// No-op sink. Useful for tests and for the platform-contracts
/// fixture matrix.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&self, _event: &AuditEvent) -> Result<(), SinkError> {
        Ok(())
    }
}

/// In-memory sink for tests. Records every event into a `Vec`
/// guarded by a `Mutex`.
#[derive(Debug, Default)]
pub struct InMemoryAuditSink {
    events: Mutex<Vec<AuditEvent>>,
}

impl InMemoryAuditSink {
    /// Construct a new in-memory sink.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot the recorded events.
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events
            .lock()
            .expect("InMemoryAuditSink mutex poisoned")
            .clone()
    }

    /// Number of recorded events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events
            .lock()
            .expect("InMemoryAuditSink mutex poisoned")
            .len()
    }

    /// True if no events have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AuditSink for InMemoryAuditSink {
    fn record(&self, event: &AuditEvent) -> Result<(), SinkError> {
        self.events
            .lock()
            .expect("InMemoryAuditSink mutex poisoned")
            .push(event.clone());
        Ok(())
    }
}
