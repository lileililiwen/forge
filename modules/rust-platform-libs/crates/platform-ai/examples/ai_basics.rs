//! Minimal `platform-ai` chat through a stub provider.
//!
//! Mirrors `docs/usage.md`; `cargo test --workspace` keeps it
//! compiling.

use async_trait::async_trait;
use platform_ai::{
    AiContext, AiError, AiProvider, ChatRequest, ChatResponse, Content, FinishReason, Message, Role,
};
use platform_core::{CorrelationId, FixedClock};

/// Stub provider proving the provider-neutral contract: it
/// validates through the shared path and returns a fixed response.
/// No network, no cargo features required.
struct StubProvider;

#[async_trait]
impl AiProvider for StubProvider {
    fn info(&self) -> &platform_ai::ProviderInfo {
        // A `const`-ish hack: keep a single static alive for the
        // example's lifetime.
        static INFO: std::sync::OnceLock<platform_ai::ProviderInfo> = std::sync::OnceLock::new();
        INFO.get_or_init(|| {
            platform_ai::ProviderInfo::new(
                "stub",
                platform_ai::ProviderKind::Custom,
                platform_ai::Capabilities::none(),
            )
        })
    }

    async fn generate(
        &self,
        request: &ChatRequest,
        _cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError> {
        request.validate()?;
        Ok(ChatResponse::new(
            Some("hello back".into()),
            None,
            FinishReason::Stop,
            None,
            "stub",
            None,
        ))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let request = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .model("gpt-fixture")
        .build()
        .expect("valid request");

    let clock = FixedClock::at_epoch();
    let cx = AiContext::new(CorrelationId::new(), &clock);
    let response = StubProvider
        .generate(&request, &cx)
        .await
        .expect("stub accepts");
    assert_eq!(response.content(), Some("hello back"));
    assert_eq!(response.finish(), FinishReason::Stop);
    println!("provider: {} -> {}", response.model(), response.model());
}
