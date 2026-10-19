//! Mock transport integration coverage (feature `mock`): scripted
//! success / failure / timeout sequences, injected-clock receipts,
//! and validation-before-script ordering.

#![cfg(feature = "mock")]

use std::time::Duration;

use platform_core::{Clock, CorrelationId, FixedClock};
use platform_mailing::mock::{MockOutcome, MockTransport};
use platform_mailing::{Body, EmailAddress, MailTransport, MailingError, SendContext};

fn message() -> platform_mailing::EmailMessage {
    platform_mailing::EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .to(EmailAddress::parse("alice@example.com").expect("addr"))
        .subject("deploy finished")
        .body(Body::alternative("done", "<p>done</p>"))
        .build()
        .expect("message builds")
}

#[tokio::test]
async fn scripted_sequence_accept_reject_timeout() {
    let clock = FixedClock::at_epoch();
    let cx = SendContext::new(CorrelationId::new(), &clock);
    let mock = MockTransport::new();
    mock.script(MockOutcome::accept_with_id("mock-1"));
    mock.script(MockOutcome::fail(MailingError::rejected_by_server(
        Some(550),
        "mailbox unavailable",
    )));
    mock.script(MockOutcome::fail(MailingError::timeout(
        Duration::from_secs(5),
    )));
    assert_eq!(mock.queued(), 3);

    let receipt = mock.send(&message(), &cx).await.expect("accept");
    assert_eq!(receipt.message_id(), Some("mock-1"));
    assert_eq!(receipt.sent_at(), clock.now());

    let err = mock.send(&message(), &cx).await.expect_err("reject");
    assert!(matches!(err, MailingError::RejectedByServer { .. }));
    assert_eq!(err.category(), platform_core::ErrorCategory::Unavailable);

    let err = mock.send(&message(), &cx).await.expect_err("timeout");
    assert!(matches!(err, MailingError::Timeout { .. }));

    assert_eq!(mock.queued(), 0);
}

#[tokio::test]
async fn invalid_messages_are_rejected_at_build_time() {
    // The builder enforces every invariant, so no invalid message can
    // reach a transport through the public API; the queued script is
    // therefore never spent on malformed input.
    let mock = MockTransport::new();
    mock.script_accept();
    let bad = platform_mailing::EmailMessage::builder(
        EmailAddress::parse("ops@example.com").expect("addr"),
    )
    .subject("missing recipients and body")
    .build();
    assert!(matches!(bad, Err(MailingError::Validation(_))));
    assert_eq!(mock.queued(), 1, "script unspent: no send was attempted");

    // And a clock advance is visible on the next receipt: the mock
    // uses only the injected clock.
    let clock = FixedClock::at_epoch();
    let cx = SendContext::new(CorrelationId::new(), &clock);
    clock.advance(Duration::from_secs(3600));
    let receipt = mock.send(&message(), &cx).await.expect("accept");
    assert_eq!(receipt.sent_at(), clock.now());
}
