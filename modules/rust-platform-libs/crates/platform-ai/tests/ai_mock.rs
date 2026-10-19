//! Mock provider integration coverage (feature `mock`): scripted
//! success / failure sequences, capability gating, validation before
//! the script, stream chunking, and the embed path.
//!
//! These tests use the in-crate `MockAi` so a consumer test suite can
//! script provider outcomes without depending on the test-only
//! `platform-testing` crate. The mock is also the
//! `default` capability shape every other adapter must support.

#![cfg(feature = "mock")]

use futures::stream::StreamExt;
use platform_ai::mock::{MockAi, ScriptedChat, ScriptedEmbed, ScriptedStream};
use platform_ai::{
    AiContext, AiProvider, Capabilities, ChatRequest, Content, EmbeddingProvider, FinishReason,
    Message, Role,
};
use platform_core::{CorrelationId, FixedClock};

fn user_request() -> ChatRequest {
    ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .build()
        .expect("valid request")
}

fn context() -> AiContext<'static> {
    // Leak a `FixedClock` for `'static`; the test process exits at
    // the end of the run, so the leak is fine.
    let clock: &'static FixedClock = Box::leak(Box::new(FixedClock::at_epoch()));
    AiContext::new(CorrelationId::new(), clock)
}

#[tokio::test]
async fn scripted_chat_sequence_consumed_in_order() {
    let mock = MockAi::new();
    mock.script_chat(ScriptedChat::ok("first", FinishReason::Stop));
    mock.script_chat(ScriptedChat::ok_with_usage(
        "second",
        FinishReason::Length,
        12,
        34,
    ));
    mock.script_chat(ScriptedChat::Err(platform_ai::AiError::RateLimited {
        retry_after_secs: Some(1),
    }));
    assert_eq!(mock.chat_queued(), 3);

    let cx = context();

    let first = mock.generate(&user_request(), &cx).await.expect("first ok");
    assert_eq!(first.content(), Some("first"));
    assert_eq!(first.finish(), FinishReason::Stop);

    let second = mock
        .generate(&user_request(), &cx)
        .await
        .expect("second ok");
    assert_eq!(second.content(), Some("second"));
    assert_eq!(second.finish(), FinishReason::Length);
    let usage = second.usage().expect("usage reported");
    assert_eq!(usage.input_tokens(), Some(12));
    assert_eq!(usage.output_tokens(), Some(34));

    let err = mock
        .generate(&user_request(), &cx)
        .await
        .expect_err("third must fail");
    assert!(matches!(err, platform_ai::AiError::RateLimited { .. }));

    // Empty script: a default Ok with no content is returned.
    let fallback = mock
        .generate(&user_request(), &cx)
        .await
        .expect("default ok");
    assert!(fallback.content().is_none());
    assert_eq!(mock.chat_queued(), 0);
}

#[tokio::test]
async fn capability_gating_refuses_calls_above_the_mock() {
    // A mock with no STREAMING cannot stream; the trait default
    // surfaces Unsupported without consulting the script.
    let mock = MockAi::with_capabilities(Capabilities::EMBEDDINGS);
    mock.script_stream(ScriptedStream::text_deltas(
        vec!["ignored".into()],
        platform_ai::ChatResponse::new(
            Some("ignored".into()),
            None,
            FinishReason::Stop,
            None,
            "mock",
            None,
        ),
    ));
    let cx = context();
    let result = mock.stream(&user_request(), &cx).await;
    let err = match result {
        Ok(_) => panic!("stream must refuse without STREAMING"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        platform_ai::AiError::Unsupported(c) if c == Capabilities::STREAMING
    ));
    // The scripted outcome is unspent: the gating happened before
    // the script was consulted.
    assert_eq!(mock.chat_queued(), 0);
}

#[tokio::test]
async fn stream_emits_text_deltas_then_a_single_turn() {
    let mock = MockAi::new();
    let final_response = platform_ai::ChatResponse::new(
        Some("hello world".into()),
        None,
        FinishReason::Stop,
        None,
        "mock",
        None,
    );
    mock.script_stream(ScriptedStream::text_deltas(
        vec!["hello ".into(), "world".into()],
        final_response.clone(),
    ));

    let cx = context();
    let mut stream = mock
        .stream(&user_request(), &cx)
        .await
        .expect("stream accepts");
    let mut deltas: Vec<String> = Vec::new();
    let mut turn: Option<platform_ai::ChatResponse> = None;
    while let Some(item) = stream.next().await {
        match item.expect("chunk is Ok") {
            platform_ai::StreamChunk::Text { delta } => deltas.push(delta),
            platform_ai::StreamChunk::Turn(response) => turn = Some(response),
        }
    }
    assert_eq!(deltas, vec!["hello ".to_string(), "world".to_string()]);
    let turn = turn.expect("terminal Turn was emitted");
    assert_eq!(turn.content(), Some("hello world"));
    assert_eq!(turn.finish(), FinishReason::Stop);
}

#[tokio::test]
async fn script_stream_err_yields_a_single_error() {
    let mock = MockAi::new();
    mock.script_stream(ScriptedStream::Err(platform_ai::AiError::Timeout));
    let cx = context();
    let stream = mock
        .stream(&user_request(), &cx)
        .await
        .expect("stream accepts");
    // `BoxStream` is `Pin<Box<dyn Stream>>`, which `StreamExt::next`
    // can drive directly without re-pinning.
    let mut stream = Box::pin(stream);
    let mut errs: Vec<platform_ai::AiError> = Vec::new();
    while let Some(item) = stream.next().await {
        errs.push(item.expect_err("single error chunk"));
    }
    assert_eq!(errs.len(), 1);
    assert!(matches!(errs[0], platform_ai::AiError::Timeout));
}

#[tokio::test]
async fn embed_returns_scripted_vectors() {
    let mock = MockAi::new();
    mock.script_embed(ScriptedEmbed::Ok(vec![
        vec![0.1, 0.2, 0.3],
        vec![0.4, 0.5, 0.6],
    ]));
    let cx = context();
    let vectors = mock.embed(&["a", "b"], &cx).await.expect("embed ok");
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].values(), &[0.1, 0.2, 0.3]);
    assert_eq!(vectors[1].values(), &[0.4, 0.5, 0.6]);
    assert_eq!(vectors[0].model(), "mock");
}

#[tokio::test]
async fn embed_falls_back_to_zero_vectors_when_script_is_empty() {
    let mock = MockAi::new();
    let cx = context();
    let vectors = mock
        .embed(&["one", "two", "three"], &cx)
        .await
        .expect("default embed ok");
    assert_eq!(vectors.len(), 3);
    for v in &vectors {
        assert_eq!(v.values().len(), 4);
        assert!(v.values().iter().all(|x| *x == 0.0));
    }
}

#[tokio::test]
async fn embed_refuses_an_empty_batch_before_the_script() {
    let mock = MockAi::new();
    mock.script_embed(ScriptedEmbed::Ok(vec![vec![0.0; 4]]));
    let cx = context();
    let err = mock.embed(&[], &cx).await.expect_err("empty batch fails");
    assert!(matches!(err, platform_ai::AiError::Validation(_)));
    // The scripted outcome is unspent: validation runs first.
}

#[tokio::test]
async fn embed_refuses_an_oversize_batch() {
    let mock = MockAi::new();
    let inputs: Vec<String> = (0..=platform_ai::types::MAX_EMBED_TEXTS)
        .map(|i| format!("text {i}"))
        .collect();
    let input_refs: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let cx = context();
    let err = mock
        .embed(&input_refs, &cx)
        .await
        .expect_err("oversize batch fails");
    assert!(matches!(err, platform_ai::AiError::Validation(_)));
}
