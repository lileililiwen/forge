//! Deterministic, network-free [`MailTransport`](crate::MailTransport)
//! for consumer test suites.
//!
//! The mock lives in this crate (behind the `mock` feature) so
//! production crates can use it without depending on the test-only
//! `platform-testing` crate. Outcomes are scripted explicitly; an
//! empty script accepts with no provider message id. Timestamps come
//! only from the [`SendContext`](crate::SendContext) clock, so tests
//! with a pinned clock are fully deterministic.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::MailingError;
use crate::transport::{DeliveryReceipt, MailTransport, SendContext};
use crate::types::EmailMessage;

/// One scripted mock outcome.
#[derive(Debug, Clone)]
pub enum MockOutcome {
    /// Accept the message; the receipt carries the configured
    /// provider id (or none).
    Accept {
        /// Provider message id surfaced on the receipt.
        message_id: Option<String>,
    },
    /// Fail the send with the configured error.
    Fail(MailingError),
}

impl MockOutcome {
    /// Accept with a provider message id.
    #[must_use]
    pub fn accept_with_id(id: impl Into<String>) -> Self {
        Self::Accept {
            message_id: Some(id.into()),
        }
    }

    /// Accept with no provider message id.
    #[must_use]
    pub fn accept() -> Self {
        Self::Accept { message_id: None }
    }

    /// Fail with the given error (including timeouts via
    /// [`MailingError::timeout`]).
    #[must_use]
    pub fn fail(err: MailingError) -> Self {
        Self::Fail(err)
    }
}

/// Scripted transport. Interior mutability (`Mutex`) keeps `send`
/// on `&self`, matching real transports.
#[derive(Debug, Default)]
pub struct MockTransport {
    script: Mutex<VecDeque<MockOutcome>>,
}

impl MockTransport {
    /// New mock whose unscripted sends accept with no message id.
    #[must_use]
    pub fn new() -> Self {
        Self {
            script: Mutex::new(VecDeque::new()),
        }
    }

    /// Queue one outcome, consumed by the next `send`.
    pub fn script(&self, outcome: MockOutcome) {
        self.script
            .lock()
            .expect("MockTransport script mutex poisoned")
            .push_back(outcome);
    }

    /// Queue an acceptance.
    pub fn script_accept(&self) {
        self.script(MockOutcome::accept());
    }

    /// Queue a failure.
    pub fn script_fail(&self, err: MailingError) {
        self.script(MockOutcome::fail(err));
    }

    /// Number of queued (unconsumed) outcomes.
    #[must_use]
    pub fn queued(&self) -> usize {
        self.script
            .lock()
            .expect("MockTransport script mutex poisoned")
            .len()
    }
}

#[async_trait]
impl MailTransport for MockTransport {
    async fn send(
        &self,
        msg: &EmailMessage,
        cx: &SendContext,
    ) -> Result<DeliveryReceipt, MailingError> {
        // Validation first: an invalid message never consults the script.
        msg.validate()?;
        let outcome = self
            .script
            .lock()
            .expect("MockTransport script mutex poisoned")
            .pop_front()
            .unwrap_or_else(MockOutcome::accept);
        match outcome {
            MockOutcome::Accept { message_id } => {
                Ok(DeliveryReceipt::new(message_id, cx.clock().now()))
            }
            MockOutcome::Fail(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use platform_core::{Clock, FixedClock};

    use super::*;
    use crate::types::{Body, EmailAddress};

    fn message() -> EmailMessage {
        EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
            .to(EmailAddress::parse("alice@example.com").expect("addr"))
            .subject("hello")
            .body(Body::text("hi"))
            .build()
            .expect("message builds")
    }

    #[tokio::test]
    async fn scripted_accept_then_timeout_uses_injected_clock() {
        let clock = FixedClock::at_epoch();
        let cx = SendContext::new(platform_core::CorrelationId::new(), &clock);
        let mock = MockTransport::new();
        mock.script(MockOutcome::accept_with_id("mock-1"));
        mock.script(MockOutcome::fail(MailingError::timeout(
            Duration::from_secs(5),
        )));

        let receipt = mock
            .send(&message(), &cx)
            .await
            .expect("scripted accept succeeds");
        assert_eq!(receipt.message_id(), Some("mock-1"));
        assert_eq!(receipt.sent_at(), clock.now());

        let err = mock
            .send(&message(), &cx)
            .await
            .expect_err("scripted failure fails");
        assert!(matches!(err, MailingError::Timeout { .. }));

        // The script is consumed; the next send accepts by default.
        assert_eq!(mock.queued(), 0);
        let fallback = mock.send(&message(), &cx).await.expect("default accepts");
        assert_eq!(fallback.message_id(), None);
    }

    #[tokio::test]
    async fn receipts_are_deterministic_under_a_pinned_clock() {
        let clock = FixedClock::at_epoch();
        let cx = SendContext::new(platform_core::CorrelationId::new(), &clock);
        let mock = MockTransport::new();

        let first = mock.send(&message(), &cx).await.expect("accepts");
        clock.advance(std::time::Duration::from_secs(60));
        let second = mock.send(&message(), &cx).await.expect("accepts");
        assert_ne!(first.sent_at(), second.sent_at());
        assert_eq!(second.sent_at(), clock.now());
    }
}
