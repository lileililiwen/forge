# Usage examples

One minimal, compile-checked example per production crate. Each snippet
below is the exact code of the corresponding file under
`crates/*/examples/`; `cargo test --workspace` (which builds all
examples) is the verification method. If a snippet stops compiling, it
is fixed or removed — never documented as working.

Test-only crates (`platform-testing`, `platform-arch-tests`) are
excluded: they exist to support consumer test suites and workspace
policy checks, not to be called from production code.

## `platform-core`: bounded IDs, clock, safe error

File: `crates/platform-core/examples/basics.rs`

```rust
use platform_core::{
    BoundedString, Clock, CorrelationId, ErrorCategory, FixedClock, Id, SafeError,
};

/// Phantom tag giving the opaque ID its type.
struct UserTag;

fn main() {
    // Length-bounded strings reject over-long input at construction.
    let name = BoundedString::<64>::try_new("alice").expect("short name fits");
    assert_eq!(name.as_str(), "alice");

    // Opaque, phantom-typed identifiers.
    let user_id = Id::<UserTag>::new("usr_alice");
    assert_eq!(user_id.as_str(), "usr_alice");

    // One shared "now" source; tests pin it with `FixedClock`.
    let clock = FixedClock::at_epoch();
    let _now = clock.now();

    // Correlation IDs travel with requests across crates.
    let correlation_id = CorrelationId::new();

    // Safe errors carry a stable category and never embed secrets.
    let err = SafeError::new(ErrorCategory::Unavailable, "upstream timed out")
        .with_correlation(correlation_id);
    assert_eq!(err.category(), ErrorCategory::Unavailable);
    println!("{err}");
}
```

## `platform-contracts`: envelope a typed payload

File: `crates/platform-contracts/examples/envelope.rs`

```rust
use platform_contracts::{parse_contract_identifier, Envelope, Readiness, ReadinessStatus};

fn main() {
    // Typed payloads mirror the language-neutral schema families.
    let payload = Readiness {
        service: "billing-api".to_string(),
        status: ReadinessStatus::Ready,
        checked_at: "2026-09-30T00:00:00Z".to_string(),
        checks: None,
        tenant_id: None,
    };

    // Every contract document travels inside an envelope.
    let envelope = Envelope::new(
        "platform.readiness/0.1.0",
        time::OffsetDateTime::UNIX_EPOCH,
        payload,
    );
    let document = serde_json::to_value(&envelope).expect("envelope serializes");
    assert_eq!(document["contract"], "platform.readiness/0.1.0");

    // Contract identifiers parse into a family plus a semver triple.
    let (family, version) =
        parse_contract_identifier("platform.readiness/0.1.0").expect("valid identifier");
    assert_eq!(family, "platform.readiness");
    assert_eq!((version.major, version.minor, version.patch), (0, 1, 0));
}
```

## `platform-identity`: subject plus permission check

File: `crates/platform-identity/examples/subject_permission.rs`

```rust
use platform_core::{CorrelationId, FixedClock};
use platform_identity::{
    DenyByDefaultEvaluator, Permission, PermissionEvaluator, Subject, SubjectId, SubjectType,
    TenantId,
};

fn main() {
    // The application resolves a subject from its own store; the
    // platform only sees the resulting trait shape.
    let subject = Subject::new(
        SubjectId::new("usr_alice"),
        SubjectType::User,
        TenantId::new("acme"),
    );

    // Permission keys are bounded strings mirroring the schema.
    let action = Permission::try_new("billing.read").expect("action fits bounds");

    // Reference evaluators are safe defaults, not production policy:
    // deny-by-default rejects everything.
    let clock = FixedClock::at_epoch();
    let evaluation = DenyByDefaultEvaluator.evaluate(
        &subject,
        &action,
        "invoice-42",
        subject.tenant_id(),
        &clock,
        CorrelationId::new(),
    );
    assert!(!evaluation.is_allowed());
    println!("decision: denied ({:?})", evaluation.reason());
}
```

## `platform-audit`: build an event and record it

File: `crates/platform-audit/examples/audit_event.rs`

```rust
use platform_audit::{
    sink::InMemoryAuditSink, AuditCategory, AuditEventBuilder, AuditOutcome, AuditSink,
    AuditSubjectType,
};
use platform_core::{Clock, FixedClock};

fn main() {
    let clock = FixedClock::at_epoch();
    let sink = InMemoryAuditSink::new();

    // The builder stamps the event; redaction-checked context rejects
    // secret-named keys at construction.
    let event = AuditEventBuilder::new(
        "evt-1",
        clock.now(),
        AuditCategory::Authentication,
        "authenticate",
        AuditOutcome::Success,
        "usr_alice",
        AuditSubjectType::User,
        "acme",
    )
    .with_resource("session-7")
    .with_context("ip", "203.0.113.10")
    .expect("non-secret context key accepted")
    .build();

    sink.record(&event).expect("in-memory sink accepts");
    assert_eq!(sink.len(), 1);
    println!("recorded {} event(s)", sink.len());
}
```

## `platform-mailing`: send through the provider-neutral contract

File: `crates/platform-mailing/examples/send_via_stub.rs`

```rust
use async_trait::async_trait;
use platform_core::{Clock, CorrelationId, FixedClock};
use platform_mailing::{
    Body, DeliveryReceipt, EmailAddress, EmailMessage, MailTransport, MailingError,
    SendContext,
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
    let message = EmailMessage::builder(
        EmailAddress::parse("ops@example.com").expect("sender parses"),
    )
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
```

The opt-in pieces live behind cargo features: `smtp-lettre`
(`SmtpTransportConfig` + `LettreSmtpTransport`, rustls only) and
`mock` (`MockTransport` with scripted outcomes for consumer test
suites). Default features are empty, so the contract compiles
lettre-free. The SMTP adapter refuses `Security::None` off-loopback
at construction, and every `MailingError` maps onto a shipped
`platform-core` `ErrorCategory` without leaking credentials, bodies,
or recipient local-parts.

## `platform-ai`: chat through the provider-neutral contract

File: `crates/platform-ai/examples/ai_basics.rs`

```rust
use platform_ai::mock::{MockAi, ScriptedChat};
use platform_ai::{
    AiContext, AiProvider, ChatRequest, Content, FinishReason, Message, Role,
};
use platform_core::{CorrelationId, FixedClock};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Scripted mock proves the provider-neutral contract: it
    // validates through the shared path and returns the queued
    // response. No network, no cargo features required beyond
    // `mock`. Async needs a runtime; the example uses tokio, exactly
    // as a consumer would with its own runtime.
    let mock = MockAi::new();
    mock.script_chat(ScriptedChat::ok("hello back", FinishReason::Stop));

    let request = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .model("gpt-fixture")
        .build()
        .expect("valid request");

    let clock = FixedClock::at_epoch();
    let cx = AiContext::new(CorrelationId::new(), &clock);
    let response = mock
        .generate(&request, &cx)
        .await
        .expect("mock accepts");
    assert_eq!(response.content(), Some("hello back"));
    assert_eq!(response.finish(), FinishReason::Stop);
    println!("model: {} -> {}", response.model(), response.model());
}
```

The opt-in pieces live behind cargo features: `openai-compat`
(`OpenAiCompatConfig` + `OpenAiCompatProvider`, rustls only,
`/v1/chat/completions`, `/v1/embeddings`, SSE streaming, JSON-schema
structured output, tool calls) and `ollama` (`OllamaConfig` +
`OllamaProvider`, default loopback endpoint, NDJSON streaming, no
credentials). The `mock` feature adds the in-crate `MockAi` so
production crates can script outcomes without depending on the
test-only `platform-testing` crate. Default features are empty, so
the contract compiles reqwest-free and tokio-free. The OpenAI
adapter refuses plain HTTP to non-loopback hosts at construction,
and every `AiError` maps onto a shipped `platform-core`
`ErrorCategory` without leaking API keys, prompts, completions, or
raw response bodies.

## Publishing

The seven publishable crates
(`platform-core`, `platform-contracts`, `platform-identity`,
`platform-audit`, `platform-testing`, `platform-mailing`,
`platform-ai`) are ready for crates.io publication; the workspace
declares the registry metadata, the inter-crate dependencies use
the registry-addressable `{ path = …, version = … }` form, and
the Forge-root `platform-rust-publish.yml` workflow ships the
namespaced-tag-gated, topologically ordered, idempotent publish pipeline. The internal
`platform-arch-tests` crate stays `publish = false` and is not part
of the publish set.

The first `v0.1.0` release remains BLOCKED until the module revision is
available from the published Forge repository, the owner adds the
`PUBLICATION-APPROVED` marker, and `CARGO_REGISTRY_TOKEN` is configured.
The publisher also requires exact version validation and explicit
operator approval for workflow dispatch. Until publication lands,
depending on a crate means a path or versioned Forge git dependency
(see [Adoption](#adoption) in the README), never a crates.io version.
