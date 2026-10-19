//! Native ollama adapter integration coverage (feature `ollama`):
//! in-process `wiremock` servers replay canned fixtures so the
//! `/api/chat` NDJSON and `/api/embed` JSON shapes are exercised
//! end-to-end without hitting the network.

#![cfg(feature = "ollama")]

use std::time::Duration;

use futures::stream::StreamExt;
use platform_ai::ollama::{OllamaConfig, OllamaProvider, DEFAULT_CHAT_PATH, DEFAULT_EMBED_PATH};
use platform_ai::types::{Content, Message, Role};
use platform_ai::{AiContext, AiError, AiProvider, ChatRequest};
use platform_core::{CorrelationId, FixedClock};
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE_MODEL: &str = "llama-fixture";

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

async fn provider_for(server: &MockServer) -> OllamaProvider {
    let cfg = OllamaConfig::new()
        .with_base_url(server.uri())
        .expect("base url accepted")
        .with_chat_path(DEFAULT_CHAT_PATH)
        .with_embed_path(DEFAULT_EMBED_PATH)
        .with_timeout(Duration::from_secs(5));
    OllamaProvider::new(cfg).expect("provider builds")
}

#[tokio::test]
async fn happy_path_chat_completion_round_trip() {
    let server = MockServer::start().await;
    let body = json!({
        "model": FIXTURE_MODEL,
        "message": {"role": "assistant", "content": "hello back"},
        "done": true,
        "done_reason": "stop",
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({
            "model": FIXTURE_MODEL,
            "messages": [{"role": "user", "content": "hi"}],
            "stream": false,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
}

#[tokio::test]
async fn ndjson_stream_emits_deltas_then_a_terminal_turn() {
    let server = MockServer::start().await;
    let stream_body = concat!(
        "{\"model\":\"llama-fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"hel\"},\"done\":false}\n",
        "{\"model\":\"llama-fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"lo \"},\"done\":false}\n",
        "{\"model\":\"llama-fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"world\"},\"done\":true,\"done_reason\":\"stop\"}\n",
    );
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({"stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/x-ndjson")
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
    assert_eq!(turn.model(), FIXTURE_MODEL);
}

#[tokio::test]
async fn stream_without_done_is_reported_as_malformed() {
    let server = MockServer::start().await;
    let stream_body =
        "{\"model\":\"llama-fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"hi\"},\"done\":false}\n";
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({"stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/x-ndjson")
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
    let mut text_chunks = 0usize;
    let mut last_error: Option<AiError> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(platform_ai::StreamChunk::Text { .. }) => text_chunks += 1,
            Ok(platform_ai::StreamChunk::Turn(_)) => {
                panic!("truncated stream must not emit a Turn")
            }
            Err(err) => last_error = Some(err),
        }
    }
    assert_eq!(text_chunks, 1);
    let err = last_error.expect("a MalformedResponse was yielded");
    assert!(matches!(err, AiError::MalformedResponse(_)));
}

#[tokio::test]
async fn server_error_in_stream_yields_provider_server() {
    let server = MockServer::start().await;
    let stream_body = concat!(
        "{\"model\":\"llama-fixture\",\"message\":{\"role\":\"assistant\",\"content\":\"hi\"},\"done\":false}\n",
        "{\"error\":\"model is loading\"}\n",
    );
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({"stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/x-ndjson")
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
    let mut last_error: Option<AiError> = None;
    while let Some(item) = stream.next().await {
        if let Err(err) = item {
            last_error = Some(err);
        }
    }
    let err = last_error.expect("a ProviderServer error was yielded");
    assert!(matches!(err, AiError::ProviderServer { .. }));
}

#[tokio::test]
async fn http_404_maps_to_validation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"error":"model not found"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("404 must fail");
    assert!(matches!(err, AiError::Validation(_)));
}

#[tokio::test]
async fn http_500_maps_to_provider_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .generate(&user_request(), &context())
        .await
        .expect_err("500 must fail");
    if let AiError::ProviderServer {
        status,
        safe_detail,
    } = err
    {
        assert_eq!(status, 500);
        assert_eq!(safe_detail, "internal error");
    } else {
        panic!("expected ProviderServer, got {err:?}");
    }
}

#[tokio::test]
async fn embeddings_batch_returns_one_vector_per_input() {
    let server = MockServer::start().await;
    let body = json!({
        "model": "nomic-embed-fixture",
        "embeddings": [
            [0.1, 0.2, 0.3],
            [0.4, 0.5, 0.6],
        ]
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_EMBED_PATH))
        .and(body_partial_json(json!({
            "model": "nomic-embed-fixture",
            "input": ["a", "b"],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let vectors = provider
        .embed_with_model("nomic-embed-fixture", &["a", "b"])
        .await
        .expect("embed ok");
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].values(), &[0.1, 0.2, 0.3]);
    assert_eq!(vectors[1].values(), &[0.4, 0.5, 0.6]);
    assert_eq!(vectors[0].model(), "nomic-embed-fixture");
}

#[tokio::test]
async fn embeddings_count_mismatch_is_malformed() {
    let server = MockServer::start().await;
    // Server returns one embedding for two inputs.
    let body = json!({
        "model": "nomic-embed-fixture",
        "embeddings": [[0.1, 0.2, 0.3]]
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_EMBED_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = provider_for(&server).await;
    let err = provider
        .embed_with_model("nomic-embed-fixture", &["a", "b"])
        .await
        .expect_err("count mismatch must fail");
    assert!(matches!(err, AiError::MalformedResponse(_)));
}

#[tokio::test]
async fn structured_request_sends_format_field() {
    let server = MockServer::start().await;
    let body = json!({
        "model": FIXTURE_MODEL,
        "message": {"role": "assistant", "content": "{\"answer\":42}"},
        "done": true,
        "done_reason": "stop",
    });
    Mock::given(method("POST"))
        .and(path(DEFAULT_CHAT_PATH))
        .and(body_partial_json(json!({
            "format": {"type": "object"},
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
async fn embed_with_empty_model_is_refused_before_the_socket() {
    let server = MockServer::start().await;
    let provider = provider_for(&server).await;
    let err = provider
        .embed_with_model("", &["hi"])
        .await
        .expect_err("empty model must fail");
    assert!(matches!(err, AiError::Validation(_)));
}

#[tokio::test]
async fn embed_with_empty_batch_is_refused_before_the_socket() {
    let server = MockServer::start().await;
    let provider = provider_for(&server).await;
    let err = provider
        .embed_with_model("any", &[])
        .await
        .expect_err("empty batch must fail");
    assert!(matches!(err, AiError::Validation(_)));
}
