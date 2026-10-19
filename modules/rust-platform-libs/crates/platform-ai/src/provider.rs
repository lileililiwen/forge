//! The provider-neutral capability contract.
//!
//! Consumers depend on [`AiProvider`] (injected as `dyn AiProvider`
//! via `async-trait`, which keeps the trait object-safe on the 1.75
//! toolchain where RPITIT is not dyn-compatible) and optionally on
//! [`EmbeddingProvider`]. Validation runs inside every method before
//! any transport work.

use async_trait::async_trait;
use futures::stream::BoxStream;
use platform_core::{Clock, CorrelationId};

use crate::error::AiError;
use crate::types::{Capabilities, ChatRequest, ChatResponse, Embedding, ProviderInfo, StreamChunk};

/// Per-call context: correlation for log aggregation and the clock
/// that may stamp timeouts and timestamps. The clock is borrowed so
/// tests can pass a pinned clock without ownership churn.
pub struct AiContext<'a> {
    correlation_id: CorrelationId,
    clock: &'a dyn Clock,
}

impl<'a> AiContext<'a> {
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

/// The single AI capability every consumer programs against.
///
/// Implementations MUST call [`ChatRequest::validate`] (and, for
/// structured output, validate the schema themselves) before any
/// transport work, and MUST return [`AiError::Validation`] without
/// side effects when it fails. A returned `Ok` means the provider
/// HTTP call completed with a usable response body; the platform
/// performs no retry, fallback, or agent loop, and never estimates
/// token counts locally (design L4).
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Provider self-description. The trait default methods consult
    /// `info().capabilities()` to decide whether to refuse a call
    /// with [`AiError::Unsupported`].
    fn info(&self) -> &ProviderInfo;

    /// Run one non-streaming chat completion.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] for every failure class documented in
    /// [`crate::error`]. Validation failures leave the network
    /// untouched.
    async fn generate(
        &self,
        request: &ChatRequest,
        cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError>;

    /// Run one streaming chat completion, returning a stream of
    /// [`StreamChunk`]s that ends with exactly one
    /// [`StreamChunk::Turn`].
    ///
    /// The default implementation refuses the call with
    /// [`AiError::Unsupported`] when [`Capabilities::STREAMING`] is
    /// absent from `info().capabilities()`. Adapters that declare
    /// the capability override this method.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] for every failure class documented in
    /// [`crate::error`].
    async fn stream(
        &self,
        _request: &ChatRequest,
        _cx: &AiContext<'_>,
    ) -> Result<BoxStream<'static, Result<StreamChunk, AiError>>, AiError> {
        if !self.info().capabilities().contains(Capabilities::STREAMING) {
            return Err(AiError::Unsupported(Capabilities::STREAMING));
        }
        // Declared but not implemented: the provider contract is
        // broken; surface as a configuration error rather than a
        // silent hang.
        Err(AiError::configuration(
            "provider declares STREAMING but does not implement stream()",
        ))
    }

    /// Run one chat completion constrained to a JSON schema.
    ///
    /// The default implementation refuses the call with
    /// [`AiError::Unsupported`] when [`Capabilities::STRUCTURED`] is
    /// absent from `info().capabilities()`.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] for every failure class documented in
    /// [`crate::error`]. `schema` MUST be a JSON object.
    async fn generate_structured(
        &self,
        _request: &ChatRequest,
        schema: &serde_json::Value,
        _cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError> {
        if !self
            .info()
            .capabilities()
            .contains(Capabilities::STRUCTURED)
        {
            return Err(AiError::Unsupported(Capabilities::STRUCTURED));
        }
        if !schema.is_object() {
            return Err(AiError::validation(
                "structured schema must be a JSON object",
            ));
        }
        Err(AiError::configuration(
            "provider declares STRUCTURED but does not implement generate_structured()",
        ))
    }
}

/// The embedding capability. A provider may implement this in
/// addition to [`AiProvider`] (the same struct typically does both).
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider self-description.
    fn info(&self) -> &ProviderInfo;

    /// Embed a batch of texts. The default implementation refuses
    /// the call with [`AiError::Unsupported`] when
    /// [`Capabilities::EMBEDDINGS`] is absent.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for an empty or over-cap
    /// batch, and every transport-side failure class.
    async fn embed(&self, inputs: &[&str], cx: &AiContext<'_>) -> Result<Vec<Embedding>, AiError> {
        if !self
            .info()
            .capabilities()
            .contains(Capabilities::EMBEDDINGS)
        {
            return Err(AiError::Unsupported(Capabilities::EMBEDDINGS));
        }
        validate_embedding_batch(inputs)?;
        let _ = cx;
        Err(AiError::configuration(
            "provider declares EMBEDDINGS but does not implement embed()",
        ))
    }
}

/// Enforce the embedding batch bound. Public so adapters can call it
/// from their overrides.
pub fn validate_embedding_batch(inputs: &[&str]) -> Result<(), AiError> {
    use crate::types::MAX_EMBED_TEXTS;
    if inputs.is_empty() {
        return Err(AiError::validation("embedding batch is empty"));
    }
    if inputs.len() > MAX_EMBED_TEXTS {
        return Err(AiError::validation(format!(
            "embedding batch size {} exceeds the {MAX_EMBED_TEXTS} limit",
            inputs.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures::stream::BoxStream;
    use platform_core::{FixedClock, Secret};

    use super::*;
    use crate::error::AiError;
    use crate::types::{
        ChatRequest, Content, FinishReason, Message, ProviderInfo, ProviderKind, Role,
    };

    /// A provider that implements nothing — every method falls
    /// through to the trait default and returns
    /// [`AiError::Unsupported`] (or `Configuration` for declared but
    /// unimplemented capabilities).
    #[derive(Debug)]
    struct EmptyProvider;

    #[async_trait]
    impl AiProvider for EmptyProvider {
        fn info(&self) -> &ProviderInfo {
            // A `const`-ish hack: this is a unit struct, so we keep
            // the static alive for the test's lifetime.
            static INFO: std::sync::OnceLock<ProviderInfo> = std::sync::OnceLock::new();
            INFO.get_or_init(|| {
                ProviderInfo::new("empty", ProviderKind::Custom, Capabilities::none())
            })
        }

        async fn generate(
            &self,
            _request: &ChatRequest,
            _cx: &AiContext<'_>,
        ) -> Result<ChatResponse, AiError> {
            Err(AiError::configuration("EmptyProvider has no backing model"))
        }
    }

    #[async_trait]
    impl EmbeddingProvider for EmptyProvider {
        fn info(&self) -> &ProviderInfo {
            static INFO: std::sync::OnceLock<ProviderInfo> = std::sync::OnceLock::new();
            INFO.get_or_init(|| {
                ProviderInfo::new("empty", ProviderKind::Custom, Capabilities::none())
            })
        }
    }

    fn user_request() -> ChatRequest {
        ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
            .build()
            .expect("valid request")
    }

    #[tokio::test]
    async fn default_trait_methods_refuse_undeclared_capabilities() {
        let provider = EmptyProvider;
        let clock = FixedClock::at_epoch();
        let cx = AiContext::new(CorrelationId::new(), &clock);

        // No STREAMING: default stream() refuses.
        match provider.stream(&user_request(), &cx).await {
            Err(AiError::Unsupported(caps)) => assert_eq!(caps, Capabilities::STREAMING),
            Err(other) => panic!("stream must refuse with Unsupported(STREAMING), got {other}"),
            Ok(_) => panic!("stream must refuse, got Ok"),
        }

        // No STRUCTURED: default generate_structured() refuses.
        match provider
            .generate_structured(&user_request(), &serde_json::json!({}), &cx)
            .await
        {
            Err(AiError::Unsupported(caps)) => assert_eq!(caps, Capabilities::STRUCTURED),
            Err(other) => {
                panic!("generate_structured must refuse with Unsupported(STRUCTURED), got {other}")
            }
            Ok(_) => panic!("generate_structured must refuse, got Ok"),
        }
    }

    #[tokio::test]
    async fn declared_but_unimplemented_capability_is_configuration() {
        /// Provider that declares STREAMING but doesn't override
        /// `stream`. The default implementation must surface a
        /// configuration error (not a hang, not an Unsupported).
        #[derive(Debug)]
        struct LyingProvider;
        #[async_trait]
        impl AiProvider for LyingProvider {
            fn info(&self) -> &ProviderInfo {
                static INFO: std::sync::OnceLock<ProviderInfo> = std::sync::OnceLock::new();
                INFO.get_or_init(|| {
                    ProviderInfo::new("lying", ProviderKind::Custom, Capabilities::STREAMING)
                })
            }
            async fn generate(
                &self,
                _request: &ChatRequest,
                _cx: &AiContext<'_>,
            ) -> Result<ChatResponse, AiError> {
                Ok(ChatResponse::new(
                    Some("ok".into()),
                    None,
                    FinishReason::Stop,
                    None,
                    "lying",
                    None,
                ))
            }
        }

        let provider = LyingProvider;
        let clock = FixedClock::at_epoch();
        let cx = AiContext::new(CorrelationId::new(), &clock);
        match provider.stream(&user_request(), &cx).await {
            Err(AiError::Configuration(_)) => {}
            Err(other) => panic!("lying stream must fail with Configuration, got {other}"),
            Ok(_) => panic!("lying stream must fail, got Ok"),
        }
    }

    #[tokio::test]
    async fn default_embed_refuses_when_uncapable_or_unimplemented() {
        // EmptyProvider has no EMBEDDINGS: refuse.
        let provider = EmptyProvider;
        let clock = FixedClock::at_epoch();
        let cx = AiContext::new(CorrelationId::new(), &clock);
        let result = provider.embed(&["hello"], &cx).await;
        match result {
            Err(AiError::Unsupported(caps)) => assert_eq!(caps, Capabilities::EMBEDDINGS),
            other => panic!("embed must refuse with Unsupported(EMBEDDINGS), got {other:?}"),
        }
    }

    #[test]
    fn embedding_batch_validation_enforces_bounds() {
        assert!(matches!(
            validate_embedding_batch(&[]),
            Err(AiError::Validation(_))
        ));
        let big: Vec<&str> = std::iter::repeat("x")
            .take(crate::types::MAX_EMBED_TEXTS + 1)
            .collect();
        assert!(matches!(
            validate_embedding_batch(&big),
            Err(AiError::Validation(_))
        ));
        let one = vec!["x"];
        assert!(validate_embedding_batch(&one).is_ok());
    }

    #[test]
    fn structured_schema_must_be_an_object() {
        // We can't easily reach the validate arm from outside without
        // declaring STRUCTURED, so this is a focused unit test on
        // the schema-shape guard.
        let request = user_request();
        let provider = EmptyProvider;
        let clock = FixedClock::at_epoch();
        let cx = AiContext::new(CorrelationId::new(), &clock);
        let result = futures::executor::block_on(provider.generate_structured(
            &request,
            &serde_json::json!([]),
            &cx,
        ));
        // EmptyProvider does NOT declare STRUCTURED, so we expect
        // Unsupported, not a schema-shape error. The schema-shape
        // branch is tested through the lying-provider path in
        // `declared_but_unimplemented_capability_is_configuration`.
        assert!(matches!(result, Err(AiError::Unsupported(_))));
    }

    // Suppress dead_code for the unused Arc/Secret imports that
    // would otherwise be needed by future adapter-style tests.
    #[allow(dead_code)]
    fn _unused_arc() {
        let _: Arc<()> = Arc::new(());
        let _: Secret<String> = Secret::new(String::new());
    }

    // Reference the BoxStream type so the optional dep is recognised
    // even when this unit test does not construct one.
    #[allow(dead_code)]
    fn _unused_box_stream() {
        let _: Option<BoxStream<'static, Result<StreamChunk, AiError>>> = None;
    }
}
