//! The provider-neutral send contract.
//!
//! Consumers depend on [`MailTransport`] (injected as `dyn`
//! `MailTransport` via `async-trait`, which keeps the trait
//! object-safe on the 1.75 toolchain where RPITIT is not
//! dyn-compatible). Validation runs inside every `send`
//! implementation before any transport work.

use async_trait::async_trait;
use time::OffsetDateTime;

use platform_core::{Clock, CorrelationId};

use crate::error::MailingError;
use crate::types::EmailMessage;

/// Per-send context: correlation for log aggregation and the clock
/// that stamps the receipt. The clock is borrowed so tests can pass
/// a pinned clock without ownership churn.
pub struct SendContext<'a> {
    correlation_id: CorrelationId,
    clock: &'a dyn Clock,
}

impl<'a> SendContext<'a> {
    /// Build a context from an explicit correlation id and clock.
    #[must_use]
    pub fn new(correlation_id: CorrelationId, clock: &'a dyn Clock) -> Self {
        Self {
            correlation_id,
            clock,
        }
    }

    /// Borrow the correlation id.
    #[must_use]
    pub fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }

    /// Borrow the clock.
    #[must_use]
    pub fn clock(&self) -> &dyn Clock {
        self.clock
    }
}

/// Proof that the server accepted the message for relay — not proof
/// of delivery. SMTP has no per-recipient receipt, so there is
/// exactly one result per message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryReceipt {
    message_id: Option<String>,
    sent_at: OffsetDateTime,
}

impl DeliveryReceipt {
    /// Build a receipt. `message_id` is the provider-supplied id when
    /// the transport surfaces one (SMTP does not; the mock may).
    #[must_use]
    pub fn new(message_id: Option<String>, sent_at: OffsetDateTime) -> Self {
        Self {
            message_id,
            sent_at,
        }
    }

    /// Borrow the provider message id, if any.
    #[must_use]
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// The send timestamp, taken from the [`SendContext`] clock.
    #[must_use]
    pub fn sent_at(&self) -> OffsetDateTime {
        self.sent_at
    }
}

/// The single mail capability every consumer programs against.
///
/// Implementations MUST call [`EmailMessage::validate`] first and
/// return [`MailingError::Validation`] without side effects when it
/// fails. A returned `Ok` means the server accepted the message for
/// relay. No implementation retries, dedupes, persists, or queues:
/// those are caller concerns.
#[async_trait]
pub trait MailTransport: Send + Sync {
    /// Send one message.
    ///
    /// # Errors
    ///
    /// Returns [`MailingError`] for validation failures (transport
    /// never invoked) and for every transport-side failure class.
    async fn send(
        &self,
        msg: &EmailMessage,
        cx: &SendContext,
    ) -> Result<DeliveryReceipt, MailingError>;
}
