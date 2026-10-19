//! Minimal `platform-mailing` send through a stub transport.
//!
//! Mirrors `docs/usage.md`; `cargo test --workspace` keeps it compiling.

use async_trait::async_trait;
use platform_core::{Clock, CorrelationId, FixedClock};
use platform_mailing::{
    Body, DeliveryReceipt, EmailAddress, EmailMessage, MailTransport, MailingError, SendContext,
};

/// Stub transport proving the provider-neutral contract: it
/// validates through the shared path and stamps the receipt from
/// the injected clock. No network, no cargo features required.
/// Async needs a runtime; the example uses tokio, exactly as a
/// consumer would with its own runtime.
struct StubTransport;

#[async_trait]
impl MailTransport for StubTransport {
    async fn send(
        &self,
        msg: &EmailMessage,
        cx: &SendContext,
    ) -> Result<DeliveryReceipt, MailingError> {
        msg.validate()?;
        Ok(DeliveryReceipt::new(None, cx.clock().now()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let message =
        EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("sender parses"))
            .to(EmailAddress::parse("alice@example.com").expect("recipient parses"))
            .subject("deploy finished")
            .body(Body::text("all checks green"))
            .build()
            .expect("message validates");

    let clock = FixedClock::at_epoch();
    let cx = SendContext::new(CorrelationId::new(), &clock);
    let receipt = StubTransport
        .send(&message, &cx)
        .await
        .expect("stub accepts");
    assert_eq!(receipt.sent_at(), clock.now());
    println!("accepted for relay at {}", receipt.sent_at());
}
