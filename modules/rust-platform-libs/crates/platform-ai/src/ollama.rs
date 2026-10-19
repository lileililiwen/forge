//! Native ollama HTTP adapter (feature `ollama`).
//!
//! Ollama exposes a different wire shape from the OpenAI-compatible
//! family: chat uses `/api/chat` (NDJSON streaming) and embeddings
//! use `/api/embed`. The adapter defaults to the loopback endpoint
//! and carries no credential material in its configuration. JSON-
//! schema structured output is requested via the per-call
//! `"format": <schema>` field rather than a `response_format`
//! object.

#![cfg(feature = "ollama")]

use std::time::Duration;

use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{truncate_safe_detail, AiError};
use crate::provider::{AiContext, AiProvider, EmbeddingProvider};
use crate::types::{
    Capabilities, ChatRequest, ChatResponse, Embedding, FinishReason, ProviderInfo, ProviderKind,
    StreamChunk,
};

/// Default ollama base URL.
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";
/// Default chat path.
pub const DEFAULT_CHAT_PATH: &str = "/api/chat";
/// Default embed path.
pub const DEFAULT_EMBED_PATH: &str = "/api/embed";
/// Default per-call timeout. Local models can be slow, so the
/// default is generous.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Adapter configuration. By design, ollama carries no
/// credentials; the only secret here would be a remote base URL,
/// which is itself a configuration hazard.
#[derive(Clone)]
pub struct OllamaConfig {
    base_url: String,
    chat_path: String,
    embed_path: String,
    timeout: Duration,
}

impl std::fmt::Debug for OllamaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaConfig")
            .field("base_url", &self.base_url)
            .field("chat_path", &self.chat_path)
            .field("embed_path", &self.embed_path)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl OllamaConfig {
    /// Build a config with the default loopback base URL.
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            chat_path: DEFAULT_CHAT_PATH.to_string(),
            embed_path: DEFAULT_EMBED_PATH.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Override the base URL. Loopback HTTP is accepted; remote
    /// HTTP is refused.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Configuration`] for an empty host or
    /// plain HTTP to a non-loopback host.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Result<Self, AiError> {
        let url = base_url.into();
        if url.trim().is_empty() {
            return Err(AiError::configuration("ollama base_url is empty"));
        }
        // Ollama's protocol is HTTP. Mirror the openai-compat
        // loopback-only rule for consistency: refuse plain HTTP to
        // a non-loopback host (callers behind a reverse proxy should
        // use HTTPS at the proxy, not here).
        if let Some(rest) = url.strip_prefix("http://") {
            let host = rest.split('/').next().unwrap_or("");
            let host = host.split(':').next().unwrap_or("");
            let lower = host.to_ascii_lowercase();
            let loopback = lower == "localhost"
                || lower == "127.0.0.1"
                || lower.starts_with("127.")
                || lower == "::1";
            if !loopback {
                return Err(AiError::configuration(
                    "ollama base_url must be https or loopback http",
                ));
            }
        } else if !url.starts_with("https://") {
            return Err(AiError::configuration(
                "ollama base_url must start with http:// (loopback) or https://",
            ));
        }
        self.base_url = url;
        Ok(self)
    }

    /// Override the chat path.
    #[must_use]
    pub fn with_chat_path(mut self, path: impl Into<String>) -> Self {
        self.chat_path = path.into();
        self
    }

    /// Override the embed path.
    #[must_use]
    pub fn with_embed_path(mut self, path: impl Into<String>) -> Self {
        self.embed_path = path.into();
        self
    }

    /// Override the per-call timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Borrow the base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Borrow the chat path.
    #[must_use]
    pub fn chat_path(&self) -> &str {
        &self.chat_path
    }

    /// Borrow the embed path.
    #[must_use]
    pub fn embed_path(&self) -> &str {
        &self.embed_path
    }

    /// The per-call timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// The ollama HTTP provider.
#[derive(Debug)]
pub struct OllamaProvider {
    config: OllamaConfig,
    client: Client,
    info: ProviderInfo,
}

impl OllamaProvider {
    /// Build the provider. No network I/O happens until
    /// `generate`/`stream`/`embed`.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Configuration`] when the rustls-backed
    /// HTTP client cannot be built.
    pub fn new(config: OllamaConfig) -> Result<Self, AiError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|err| AiError::configuration(format!("could not build http client: {err}")))?;
        let info = ProviderInfo::new(
            "ollama",
            ProviderKind::Ollama,
            Capabilities::STREAMING | Capabilities::STRUCTURED | Capabilities::EMBEDDINGS,
        );
        Ok(Self {
            config,
            client,
            info,
        })
    }

    /// Borrow the config.
    #[must_use]
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }
}

// --- Wire shapes ----------------------------------------------------------

#[derive(Debug, Serialize)]
struct WireChatRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a Value>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct WireMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct WireChatResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    message: Option<WireResponseMessage>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireResponseMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Serialize)]
struct WireEmbedRequest<'a> {
    model: &'a str,
    input: &'a [&'a str],
}

#[derive(Debug, Deserialize)]
struct WireEmbedResponse {
    #[serde(default)]
    #[allow(dead_code)]
    model: String,
    #[serde(default)]
    embeddings: Vec<Vec<f32>>,
}

// --- Conversion helpers ---------------------------------------------------

fn build_wire_request<'a>(
    request: &'a ChatRequest,
    schema: Option<&'a Value>,
    stream: bool,
) -> Result<WireChatRequest<'a>, AiError> {
    let model = request.model().ok_or_else(|| {
        AiError::validation("ollama requires an explicit model id on the request")
    })?;
    let messages: Result<Vec<WireMessage<'_>>, AiError> = request
        .messages()
        .iter()
        .map(|m| {
            Ok(WireMessage {
                role: m.role().as_str(),
                content: m.content().as_str(),
            })
        })
        .collect();
    Ok(WireChatRequest {
        model,
        messages: messages?,
        format: schema,
        stream,
    })
}

fn map_finish_reason(reason: Option<&str>, done: bool) -> FinishReason {
    if !done {
        return FinishReason::Unknown;
    }
    match reason {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("load") | Some("unload") => FinishReason::Stop,
        _ => FinishReason::Stop,
    }
}

fn build_chat_response(parsed: WireChatResponse) -> Result<ChatResponse, AiError> {
    if let Some(message) = parsed.error {
        return Err(AiError::ProviderServer {
            status: 0,
            safe_detail: truncate_safe_detail(&message).into_owned(),
        });
    }
    let content = parsed
        .message
        .ok_or_else(|| AiError::malformed_response("ollama response had no message"))?
        .content;
    Ok(ChatResponse::new(
        if content.is_empty() {
            None
        } else {
            Some(content)
        },
        None,
        map_finish_reason(parsed.done_reason.as_deref(), parsed.done),
        None,
        if parsed.model.is_empty() {
            "ollama".to_string()
        } else {
            parsed.model
        },
        None,
    ))
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn generate(
        &self,
        request: &ChatRequest,
        _cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError> {
        request.validate()?;
        let body = build_wire_request(request, None, false)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::from_status(status.as_u16(), Some(&body)));
        }
        let parsed: WireChatResponse = response
            .json()
            .await
            .map_err(|err| AiError::malformed_response(format!("json decode failed: {err}")))?;
        build_chat_response(parsed)
    }

    async fn stream(
        &self,
        request: &ChatRequest,
        _cx: &AiContext<'_>,
    ) -> Result<BoxStream<'static, Result<StreamChunk, AiError>>, AiError> {
        if !self.info.capabilities().contains(Capabilities::STREAMING) {
            return Err(AiError::Unsupported(Capabilities::STREAMING));
        }
        request.validate()?;
        let body = build_wire_request(request, None, true)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::from_status(status.as_u16(), Some(&body)));
        }
        let stream = parse_ndjson_stream(response);
        Ok(Box::pin(stream))
    }

    async fn generate_structured(
        &self,
        request: &ChatRequest,
        schema: &Value,
        _cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError> {
        if !self.info.capabilities().contains(Capabilities::STRUCTURED) {
            return Err(AiError::Unsupported(Capabilities::STRUCTURED));
        }
        if !schema.is_object() {
            return Err(AiError::validation(
                "structured schema must be a JSON object",
            ));
        }
        request.validate()?;
        let body = build_wire_request(request, Some(schema), false)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::from_status(status.as_u16(), Some(&body)));
        }
        let parsed: WireChatResponse = response
            .json()
            .await
            .map_err(|err| AiError::malformed_response(format!("json decode failed: {err}")))?;
        build_chat_response(parsed)
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn embed(&self, inputs: &[&str], _cx: &AiContext<'_>) -> Result<Vec<Embedding>, AiError> {
        crate::provider::validate_embedding_batch(inputs)?;
        // The model must be set explicitly; we use embed_with_model.
        Err(AiError::configuration(
            "OllamaProvider::embed requires an explicit model; call embed_with_model instead",
        ))
    }
}

impl OllamaProvider {
    /// Embed a batch of texts under a specific model id.
    pub async fn embed_with_model(
        &self,
        model: &str,
        inputs: &[&str],
    ) -> Result<Vec<Embedding>, AiError> {
        crate::provider::validate_embedding_batch(inputs)?;
        if model.is_empty() {
            return Err(AiError::validation("embedding model is empty"));
        }
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.embed_path
        );
        let body = WireEmbedRequest {
            model,
            input: inputs,
        };
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::from_status(status.as_u16(), Some(&body)));
        }
        let parsed: WireEmbedResponse = response.json().await.map_err(|err| {
            AiError::malformed_response(format!("embed json decode failed: {err}"))
        })?;
        let count = parsed.embeddings.len();
        if count != inputs.len() {
            return Err(AiError::malformed_response(format!(
                "ollama returned {count} embeddings for {} inputs",
                inputs.len()
            )));
        }
        Ok(parsed
            .embeddings
            .into_iter()
            .map(|values| Embedding::new(values, model))
            .collect())
    }
}

// --- HTTP error helpers ---------------------------------------------------

fn map_request_error(err: reqwest::Error) -> AiError {
    if err.is_timeout() {
        AiError::Timeout
    } else if err.is_connect() || err.is_request() {
        AiError::transport_unavailable(format!("http request failed: {err}"))
    } else {
        AiError::transport_unavailable(format!("http transport error: {err}"))
    }
}

// --- NDJSON stream parser -------------------------------------------------
//
// Ollama streams one JSON object per line, with `done: true` on the
// final line. We accumulate `message.content` deltas and emit
// exactly one `Turn(ChatResponse)` when the final line arrives.

fn parse_ndjson_stream(
    response: reqwest::Response,
) -> impl futures::stream::Stream<Item = Result<StreamChunk, AiError>> + Send + 'static {
    async_stream::stream! {
        use futures::stream::StreamExt;
        let mut bytes_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut accumulated = String::new();
        let mut last_model = String::new();
        let mut last_reason: Option<String> = None;
        let mut last_done = false;
        let mut saw_any = false;
        let mut error_message: Option<String> = None;

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(err) => {
                    yield Err(map_request_error(err));
                    return;
                }
            };
            buffer.extend_from_slice(&chunk);
            while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=pos).collect();
                let line = match std::str::from_utf8(&line[..line.len().saturating_sub(1)]) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<WireChatResponse>(line) {
                    Ok(parsed) => {
                        saw_any = true;
                        if !parsed.model.is_empty() {
                            last_model = parsed.model;
                        }
                        if let Some(err) = parsed.error {
                            error_message = Some(err);
                        }
                        if let Some(message) = parsed.message {
                            if !message.content.is_empty() {
                                accumulated.push_str(&message.content);
                                yield Ok(StreamChunk::Text { delta: message.content });
                            }
                        }
                        if let Some(reason) = parsed.done_reason {
                            last_reason = Some(reason);
                        }
                        if parsed.done {
                            last_done = true;
                        }
                    }
                    Err(err) => {
                        yield Err(AiError::malformed_response(format!("ndjson line invalid: {err}")));
                        return;
                    }
                }
            }
        }
        if let Some(message) = error_message {
            yield Err(AiError::ProviderServer {
                status: 0,
                safe_detail: truncate_safe_detail(&message).into_owned(),
            });
            return;
        }
        if !saw_any {
            yield Err(AiError::malformed_response("stream-truncated: empty body"));
            return;
        }
        if last_done {
            let response = ChatResponse::new(
                if accumulated.is_empty() { None } else { Some(accumulated) },
                None,
                map_finish_reason(last_reason.as_deref(), true),
                None,
                if last_model.is_empty() { "ollama".to_string() } else { last_model },
                None,
            );
            yield Ok(StreamChunk::Turn(response));
        } else {
            yield Err(AiError::malformed_response("stream-truncated: response ended without done=true"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_targets_loopback() {
        let cfg = OllamaConfig::new();
        assert_eq!(cfg.base_url(), "http://127.0.0.1:11434");
        assert_eq!(cfg.chat_path(), "/api/chat");
        assert_eq!(cfg.embed_path(), "/api/embed");
    }

    #[test]
    fn remote_http_base_url_is_refused() {
        let err = OllamaConfig::new()
            .with_base_url("http://example.com")
            .expect_err("remote http must fail");
        assert!(matches!(err, AiError::Configuration(_)));
    }

    #[test]
    fn loopback_http_is_accepted() {
        OllamaConfig::new()
            .with_base_url("http://localhost:11434")
            .expect("loopback constructs");
        OllamaConfig::new()
            .with_base_url("http://127.0.0.1:11434")
            .expect("loopback constructs");
    }

    #[test]
    fn https_is_accepted() {
        OllamaConfig::new()
            .with_base_url("https://ollama.example.com")
            .expect("https constructs");
    }

    #[test]
    fn empty_base_url_is_refused() {
        let err = OllamaConfig::new()
            .with_base_url("")
            .expect_err("empty base_url must fail");
        assert!(matches!(err, AiError::Configuration(_)));
    }

    #[test]
    fn ollama_provider_uses_ollama_kind() {
        let provider = OllamaProvider::new(OllamaConfig::new()).expect("provider builds");
        let info = AiProvider::info(&provider);
        assert_eq!(info.provider_id(), "ollama");
        assert_eq!(info.kind(), ProviderKind::Ollama);
        assert!(info.capabilities().contains(Capabilities::EMBEDDINGS));
        assert!(info.capabilities().contains(Capabilities::STREAMING));
        assert!(!info.capabilities().contains(Capabilities::TOOLS));
    }
}
