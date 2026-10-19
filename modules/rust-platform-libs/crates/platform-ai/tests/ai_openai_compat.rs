//! OpenAI-compatible HTTP adapter integration coverage (feature
//! `openai-compat`): in-process `wiremock` servers replay canned
//! fixtures so the wire shape is exercised end-to-end without
//! hitting the network.
//!
//! Coverage: happy chat completion, tool-call round-trip, SSE
//! streaming with `[DONE]`, structured json_schema body, embeddings
//! batch, plus negatives 401/429/500/malformed and the
//! content-policy classification. The fixture secrets below are
//! redacted in any error message that escapes the adapter.

#![cfg(feature = "openai-compat")]

use std::time::Duration;

use futures::stream::StreamExt;
use platform_ai::openai_compat::{
    OpenAiCompatConfig, OpenAiCompatProvider, DEFAULT_CHAT_PATH, DEFAULT_EMBED_PATH,
};
use platform_ai::types::{Content, Message, Role};
use platform_ai::{AiContext, AiError, AiProvider, ChatRequest};
use platform_core::{CorrelationId, FixedClock, Secret};
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE_API_KEY: &str = "sk-fixture-1234567890abcdef";
const FIXTURE_REQUEST_ID: &str = "req-fixture-1234";
const FIXTURE_MODEL: &str = "gpt-fixture";

fn context() -> AiContext<'static> {
    let clock: &'static FixedClock = Box::leak(Box::new(FixedClock::at_epoch()));
    AiContext::new(CorrelationId::new(), clock)
}

fn user_request() -> ChatRequest {
    ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .model(FIXTURE_MODEL)
        .build()
        .expect("valid request")
}

async fn provider_for(server: &MockServer) -> OpenAiCompatProvider {
    let cfg = OpenAiCompatConfig::with_base_url_and_id(
        Secret::new(FIXTURE_API_KEY.into()),
        server.uri(),
        "openai",
    )
    .expect("base url accepted")
    .with_chat_path(DEFAULT_CHAT_PATH)
    .with_embed_path(DEFAULT_EMBED_PATH)
    .with_timeout(Duration::from_secs(5));
    OpenAiCompatProvider::new(cfg).expect("provider builds")
}

#[tokio::test]
async fn happy_path_chat_completion_round_trip() {
    let server = MockServer::start().await;
    let response_body = json!({
        "id": FIXTURE_REQUEST_ID,
        "model": FIXTURE_MODEL,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "hello back",
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 7,
            "completion_tokens": 3,
        }
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(header("authorization", format!("Bearer {FIXTURE_API_KEY}")))
        .and(body_partial_json(json!({
            "model": FIXTURE_MODEL,
            "messages": [{"role": "user", "content": "hi"}],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let response = provider
        .generate(&user_request(), &context())
        .await
        .expect("chat ok");
    assert_eq!(response.content(), Some("hello back"));
    assert_eq!(response.finish(), platform_ai::FinishReason::Stop);
    assert_eq!(response.model(), FIXTURE_MODEL);
    assert_eq!(response.provider_request_id(), Some(FIXTURE_REQUEST_ID));
    let usage = response.usage().expect("usage reported");
    assert_eq!(usage.input_tokens(), Some(7));
    assert_eq!(usage.output_tokens(), Some(3));
}

#[tokio::test]
async fn tool_call_arguments_are_parsed_into_json() {
    let server = MockServer::start().await;
    let response_body = json!({
        "id": FIXTURE_REQUEST_ID,
        "model": FIXTURE_MODEL,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": r#"{"city":"Berlin"}"#,
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }]
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let response = provider
        .generate(&user_request(), &context())
        .await
        .expect("tool call ok");
    assert_eq!(response.finish(), platform_ai::FinishReason::ToolCalls);
    let calls = response.tool_calls().expect("tool calls present");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id(), "call_1");
    assert_eq!(calls[0].name(), "get_weather");
    assert_eq!(calls[0].arguments(), &json!({"city": "Berlin"}));
}

#[tokio::test]
async fn sse_stream_emits_deltas_then_a_terminal_turn() {
    let server = MockServer::start().await;
    let stream_body = concat!(
        "data: {\"id\":\"req-stream\",\"model\":\"gpt-fixture\",\"choices\":[{\"index\":0,\"message\":{\"content\":\"hel\"}}]}\n\n",
        "data: {\"id\":\"req-stream\",\"model\":\"gpt-fixture\",\"choices\":[{\"index\":0,\"message\":{\"content\":\"lo \"}}]}\n\n",
        "data: {\"id\":\"req-stream\",\"model\":\"gpt-fixture\",\"choices\":[{\"index\":0,\"message\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(header("accept", "text/event-stream"))
        .and(body_partial_json(json!({"stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(stream_body),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let mut stream = provider
        .stream(&user_request(), &context())
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
    assert_eq!(deltas, vec!["hel", "lo ", "world"]);
    let turn = turn.expect("terminal Turn was emitted");
    assert_eq!(turn.content(), Some("hello world"));
    assert_eq!(turn.finish(), platform_ai::FinishReason::Stop);
    assert_eq!(turn.provider_request_id(), Some("req-stream"));
}

#[tokio::test]
async fn structured_request_sends_json_schema_response_format() {
    let server = MockServer::start().await;
    let response_body = json!({
        "id": FIXTURE_REQUEST_ID,
        "model": FIXTURE_MODEL,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "{\"answer\":42}",
            },
            "finish_reason": "stop",
        }]
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": FIXTURE_MODEL,
                    "schema": {"type": "object"},
                    "strict": true,
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let schema = json!({"type": "object"});
    let response = provider
        .generate_structured(&user_request(), &schema, &context())
        .await
        .expect("structured ok");
    assert_eq!(response.content(), Some("{\"answer\":42}"));
    assert_eq!(response.finish(), platform_ai::FinishReason::Stop);
}

#[tokio::test]
async fn embeddings_batch_returns_one_vector_per_input() {
    let server = MockServer::start().await;
    let response_body = json!({
        "model": "text-embedding-fixture",
        "data": [
            {"embedding": [0.1, 0.2, 0.3], "index": 0},
            {"embedding": [0.4, 0.5, 0.6], "index": 1},
        ]
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_EMBED_PATH))
        .and(body_partial_json(json!({
            "model": "text-embedding-fixture",
            "input": ["a", "b"],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let vectors = provider
        .embed_with_model("text-embedding-fixture", &["a", "b"])
        .await
        .expect("embed ok");
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].values(), &[0.1, 0.2, 0.3]);
    assert_eq!(vectors[1].values(), &[0.4, 0.5, 0.6]);
    assert_eq!(vectors[0].model(), "text-embedding-fixture");
}

#[tokio::test]
async fn http_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"error":{"message":"bad key"}}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("401 must fail");
    assert!(matches!(err, AiError::Unauthorized));
    assert_eq!(err.category(), platform_core::ErrorCategory::Unavailable);
    // Display never echoes the bearer key, even when the server
    // reports an account id in the body.
    let rendered = format!("{err}");
    assert!(
        !rendered.contains(FIXTURE_API_KEY),
        "key leaked: {rendered}"
    );
}

#[tokio::test]
async fn http_429_maps_to_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "5")
                .set_body_string(r#"{"error":{"message":"slow down"}}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("429 must fail");
    assert!(matches!(err, AiError::RateLimited { .. }));
}

#[tokio::test]
async fn http_500_with_content_refusal_body_promotes_to_content_policy() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"error":{"message":"content_refused: too long"}}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("500 must fail");
    assert!(matches!(err, AiError::ContentPolicy(_)));
    assert_eq!(err.category(), platform_core::ErrorCategory::Validation);
}

#[tokio::test]
async fn http_500_without_content_refusal_falls_back_to_provider_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(500)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"error":{"message":"oops"}}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("500 must fail");
    if let AiError::ProviderServer { status, .. } = err {
        assert_eq!(status, 500);
    } else {
        panic!("expected ProviderServer, got {err:?}");
    }
}

#[tokio::test]
async fn malformed_json_response_maps_to_malformed_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string("not json at all"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("bad json must fail");
    assert!(matches!(err, AiError::MalformedResponse(_)));
}

#[tokio::test]
async fn stream_truncation_without_done_is_reported_as_malformed() {
    let server = MockServer::start().await;
    let stream_body = "data: {\"id\":\"req-stream\",\"model\":\"gpt-fixture\",\"choices\":[{\"index\":0,\"message\":{\"content\":\"hi\"}}]}\n\n";
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(header("accept", "text/event-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(stream_body),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let mut stream = provider
        .stream(&user_request(), &context())
        .await
        .expect("stream accepts");
    let mut text_chunks: Vec<platform_ai::StreamChunk> = Vec::new();
    let mut last_error: Option<AiError> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => text_chunks.push(chunk),
            Err(err) => last_error = Some(err),
        }
    }
    // We saw exactly one Text delta; the terminal [DONE] never
    // arrived so the stream ends with a MalformedResponse error.
    assert_eq!(text_chunks.len(), 1);
    assert!(matches!(
        text_chunks[0],
        platform_ai::StreamChunk::Text { .. }
    ));
    let err = last_error.expect("a MalformedResponse was yielded");
    assert!(matches!(err, AiError::MalformedResponse(_)));
}

#[tokio::test]
async fn embed_with_empty_model_is_refused_before_the_socket() {
    let server = MockServer::start().await;
    let provider = provider_for(&server).await;
    let err = provider
        .embed_with_model("", &["hi"])
        .await
        .expect_err("empty model must fail");
    assert!(matches!(err, AiError::Validation(_)));
}
