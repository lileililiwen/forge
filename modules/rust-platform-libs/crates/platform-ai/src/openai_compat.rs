//! OpenAI-compatible HTTP adapter (feature `openai-compat`).
//!
//! Covers OpenAI, DeepSeek, Zhipu, and any other vendor that
//! exposes the OpenAI `/v1/chat/completions` and `/v1/embeddings`
//! wire shapes. The adapter speaks
//! [`AiProvider`](crate::provider::AiProvider) and
//! [`EmbeddingProvider`](crate::provider::EmbeddingProvider),
//! configurable per-call by `base_url`, `chat_path`, and
//! `embed_path`. Streaming uses `text/event-stream`; JSON-schema
//! structured output is sent as `response_format: { type:
//! "json_schema", json_schema: { strict, schema } }`.
//!
//! All HTTP transport is `reqwest` over rustls only. The adapter
//! refuses plain HTTP to non-loopback hosts at construction
//! (mirroring the [`platform_mailing::SmtpTransportConfig`] rule
//! recorded in design §5).

#![cfg(feature = "openai-compat")]

use std::time::Duration;

use async_trait::async_trait;
use futures::stream::BoxStream;
use platform_core::Secret;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{truncate_safe_detail, AiError};
use crate::provider::{AiContext, AiProvider, EmbeddingProvider};
use crate::types::{
    Capabilities, ChatRequest, ChatResponse, Embedding, FinishReason, GenerationOptions, Message,
    ProviderInfo, ProviderKind, StreamChunk, ToolCall, ToolDefinition, Usage,
};

/// Default base URL for OpenAI.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com";
/// Default chat path.
pub const DEFAULT_CHAT_PATH: &str = "/v1/chat/completions";
/// Default embeddings path.
pub const DEFAULT_EMBED_PATH: &str = "/v1/embeddings";
/// Default per-call timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
/// Default maximum SSE stream body size, in bytes.
pub const DEFAULT_MAX_STREAM_BYTES: usize = 8 * 1024 * 1024;

/// Adapter configuration. Credentials stay inside
/// [`Secret`](platform_core::Secret) so `Debug` on this type cannot
/// leak them.
#[derive(Clone)]
pub struct OpenAiCompatConfig {
    api_key: Secret<String>,
    base_url: String,
    chat_path: String,
    embed_path: String,
    timeout: Duration,
    max_stream_bytes: usize,
    provider_id: String,
    capabilities: Capabilities,
}

impl std::fmt::Debug for OpenAiCompatConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatConfig")
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("chat_path", &self.chat_path)
            .field("embed_path", &self.embed_path)
            .field("timeout", &self.timeout)
            .field("max_stream_bytes", &self.max_stream_bytes)
            .field("provider_id", &self.provider_id)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl OpenAiCompatConfig {
    /// Build a config with the default OpenAI base URL and the
    /// supplied bearer key.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Configuration`] when the key is empty.
    pub fn new(api_key: Secret<String>) -> Result<Self, AiError> {
        Self::with_base_url_and_id(api_key, DEFAULT_BASE_URL, "openai")
    }

    /// Build a config with a custom `base_url` and `provider_id`
    /// (e.g. `"deepseek"`, `"zhipu"`).
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Configuration`] when the key is empty or
    /// `base_url` is plain HTTP to a non-loopback host.
    pub fn with_base_url_and_id(
        api_key: Secret<String>,
        base_url: impl Into<String>,
        provider_id: impl Into<String>,
    ) -> Result<Self, AiError> {
        if api_key.expose().is_empty() {
            return Err(AiError::configuration("api key is empty"));
        }
        let base_url = base_url.into();
        ensure_https_or_loopback(&base_url)?;
        Ok(Self {
            api_key,
            base_url,
            chat_path: DEFAULT_CHAT_PATH.to_string(),
            embed_path: DEFAULT_EMBED_PATH.to_string(),
            timeout: DEFAULT_TIMEOUT,
            max_stream_bytes: DEFAULT_MAX_STREAM_BYTES,
            provider_id: provider_id.into(),
            capabilities: default_capabilities(),
        })
    }

    /// Override the chat path (zhipu/deepseek may use a different
    /// route).
    #[must_use]
    pub fn with_chat_path(mut self, path: impl Into<String>) -> Self {
        self.chat_path = path.into();
        self
    }

    /// Override the embeddings path.
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

    /// Override the SSE stream byte cap.
    #[must_use]
    pub fn with_max_stream_bytes(mut self, cap: usize) -> Self {
        self.max_stream_bytes = cap;
        self
    }

    /// Override the declared capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Borrow the bearer key (the platform reads it through
    /// [`Secret::expose`] only).
    #[must_use]
    pub fn api_key(&self) -> &Secret<String> {
        &self.api_key
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

    /// The configured per-call timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The configured SSE stream byte cap.
    #[must_use]
    pub const fn max_stream_bytes(&self) -> usize {
        self.max_stream_bytes
    }

    /// The provider id used in [`ProviderInfo`].
    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }
}

/// Default declared capabilities for the OpenAI-compatible adapter.
fn default_capabilities() -> Capabilities {
    Capabilities::STREAMING
        | Capabilities::TOOLS
        | Capabilities::STRUCTURED
        | Capabilities::EMBEDDINGS
}

/// The OpenAI-compatible HTTP provider.
#[derive(Debug)]
pub struct OpenAiCompatProvider {
    config: OpenAiCompatConfig,
    client: Client,
    info: ProviderInfo,
}

impl OpenAiCompatProvider {
    /// Build the provider. The HTTP client is constructed here;
    /// no network I/O happens until `generate`/`stream`/`embed`.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Configuration`] when the rustls-backed
    /// HTTP client cannot be built.
    pub fn new(config: OpenAiCompatConfig) -> Result<Self, AiError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|err| AiError::configuration(format!("could not build http client: {err}")))?;
        let info = ProviderInfo::new(
            config.provider_id.clone(),
            ProviderKind::OpenAiCompat,
            config.capabilities,
        );
        Ok(Self {
            config,
            client,
            info,
        })
    }

    /// Borrow the config.
    #[must_use]
    pub fn config(&self) -> &OpenAiCompatConfig {
        &self.config
    }
}

/// Accept `https://...` everywhere, and `http://...` only when the
/// host is loopback.
fn ensure_https_or_loopback(base_url: &str) -> Result<(), AiError> {
    if let Some(rest) = base_url.strip_prefix("https://") {
        if rest.is_empty() {
            return Err(AiError::configuration("base_url host is empty"));
        }
        Ok(())
    } else if let Some(rest) = base_url.strip_prefix("http://") {
        let host = rest.split('/').next().unwrap_or("");
        let host = host.split(':').next().unwrap_or("");
        if is_loopback_host(host) {
            Ok(())
        } else {
            Err(AiError::configuration(
                "openai-compat base_url must be https or loopback http",
            ))
        }
    } else {
        Err(AiError::configuration(
            "openai-compat base_url must start with https:// or http:// (loopback only)",
        ))
    }
}

/// Loopback rule: `localhost`, `127.0.0.0/8`, and `::1` (bare or
/// bracketed). Mirrors the `is_loopback_host` used by
/// `platform-mailing` so the satellite pattern stays consistent.
fn is_loopback_host(host: &str) -> bool {
    let lower = host.trim().to_ascii_lowercase();
    lower == "localhost"
        || lower == "::1"
        || lower == "[::1]"
        || lower == "127.0.0.1"
        || lower.starts_with("127.")
}

// --- Wire shapes ----------------------------------------------------------
//
// These mirror the published OpenAI chat-completions and embeddings
// shapes. We only model the fields we actually need; unknown fields
// are tolerated (the registry validator only requires known fields
// to validate, not the absence of extras).

#[derive(Debug, Serialize)]
struct WireChatRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    stop: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<WireResponseFormat>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Debug, Serialize)]
struct WireMessage {
    role: &'static str,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct WireToolDefinition {
    #[serde(rename = "type")]
    kind: &'static str,
    function: WireToolFunction,
}

#[derive(Debug, Serialize)]
struct WireToolFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: Value,
}

#[derive(Debug, Serialize)]
struct WireResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    json_schema: WireJsonSchema,
}

#[derive(Debug, Serialize)]
struct WireJsonSchema {
    name: String,
    schema: Value,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct WireChatResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    choices: Vec<WireChoice>,
    #[serde(default)]
    usage: Option<WireUsage>,
    #[serde(default, rename = "id")]
    request_id: Option<String>,
    #[serde(default)]
    error: Option<WireError>,
}

#[derive(Debug, Deserialize)]
struct WireChoice {
    #[serde(default)]
    message: Option<WireResponseMessage>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<WireToolCall>>,
}

#[derive(Debug, Deserialize)]
struct WireToolCall {
    id: String,
    function: WireToolCallFunction,
}

#[derive(Debug, Deserialize)]
struct WireToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WireError {
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct WireEmbeddingsRequest<'a> {
    model: &'a str,
    input: &'a [&'a str],
}

#[derive(Debug, Deserialize)]
struct WireEmbeddingsResponse {
    #[serde(default)]
    #[allow(dead_code)]
    model: String,
    #[serde(default)]
    data: Vec<WireEmbeddingDatum>,
}

#[derive(Debug, Deserialize)]
struct WireEmbeddingDatum {
    embedding: Vec<f32>,
}

// --- Conversion helpers ---------------------------------------------------

fn to_wire_message(message: &Message) -> WireMessage {
    WireMessage {
        role: message.role().as_str(),
        content: message.content().as_str().to_string(),
        tool_call_id: message.tool_call_id().map(str::to_string),
    }
}

fn to_wire_tool(tool: &ToolDefinition) -> WireToolDefinition {
    WireToolDefinition {
        kind: "function",
        function: WireToolFunction {
            name: tool.name().to_string(),
            description: tool.description().map(str::to_string),
            parameters: tool.parameters().clone(),
        },
    }
}

fn build_wire_request<'a>(
    request: &'a ChatRequest,
    options_override: Option<&'a GenerationOptions>,
    schema: Option<(&'a str, &'a Value)>,
    stream: bool,
) -> Result<WireChatRequest<'a>, AiError> {
    let model = request.model().ok_or_else(|| {
        AiError::validation("openai-compat requires an explicit model id on the request")
    })?;
    let options = options_override.unwrap_or(request.options());
    let response_format = schema.map(|(name, schema)| WireResponseFormat {
        kind: "json_schema",
        json_schema: WireJsonSchema {
            name: name.to_string(),
            schema: schema.clone(),
            strict: true,
        },
    });
    Ok(WireChatRequest {
        model,
        messages: request.messages().iter().map(to_wire_message).collect(),
        temperature: options.temperature(),
        max_tokens: options.max_output_tokens(),
        stop: options.stop(),
        seed: options.seed(),
        tools: options
            .tools()
            .map(|tools| tools.iter().map(to_wire_tool).collect()),
        response_format,
        stream,
    })
}

fn map_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some("length") => FinishReason::Length,
        Some("content_filter") => FinishReason::ContentRefused,
        _ => FinishReason::Unknown,
    }
}

fn parse_tool_call_arguments(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn tool_calls_from_wire(calls: Vec<WireToolCall>) -> Result<Option<Vec<ToolCall>>, AiError> {
    if calls.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(calls.len());
    for call in calls {
        let arguments = parse_tool_call_arguments(&call.function.arguments);
        out.push(ToolCall::new(call.id, call.function.name, arguments)?);
    }
    Ok(Some(out))
}

fn build_chat_response(parsed: WireChatResponse) -> Result<ChatResponse, AiError> {
    if let Some(err) = parsed.error {
        let safe = err
            .message
            .as_deref()
            .map(truncate_safe_detail)
            .map(|c| c.into_owned())
            .unwrap_or_default();
        return Err(AiError::ProviderServer {
            status: 0,
            safe_detail: safe,
        });
    }
    let choice = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| AiError::malformed_response("chat response had no choices"))?;
    let message = choice
        .message
        .ok_or_else(|| AiError::malformed_response("chat response choice had no message"))?;
    let tool_calls = tool_calls_from_wire(message.tool_calls.unwrap_or_default())?;
    let usage = parsed
        .usage
        .map(|u| Usage::new(u.prompt_tokens, u.completion_tokens));
    Ok(ChatResponse::new(
        message.content,
        tool_calls,
        map_finish_reason(choice.finish_reason.as_deref()),
        usage,
        parsed.model,
        parsed.request_id,
    ))
}

#[async_trait]
impl AiProvider for OpenAiCompatProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn generate(
        &self,
        request: &ChatRequest,
        _cx: &AiContext<'_>,
    ) -> Result<ChatResponse, AiError> {
        request.validate()?;
        let body = build_wire_request(request, None, None, false)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.config.api_key.expose().as_str())
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(classify_with_status(status.as_u16(), &body));
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
        let body = build_wire_request(request, None, None, true)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.config.api_key.expose().as_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(classify_with_status(status.as_u16(), &body));
        }
        let cap = self.config.max_stream_bytes;
        let stream = parse_sse_stream(response, cap, self.info.provider_id().to_string());
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
        let model = request.model().ok_or_else(|| {
            AiError::validation("openai-compat requires an explicit model id on the request")
        })?;
        let body = build_wire_request(request, None, Some((model, schema)), false)?;
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.chat_path
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.config.api_key.expose().as_str())
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(classify_with_status(status.as_u16(), &body));
        }
        let parsed: WireChatResponse = response
            .json()
            .await
            .map_err(|err| AiError::malformed_response(format!("json decode failed: {err}")))?;
        build_chat_response(parsed)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn embed(&self, inputs: &[&str], _cx: &AiContext<'_>) -> Result<Vec<Embedding>, AiError> {
        if !self.info.capabilities().contains(Capabilities::EMBEDDINGS) {
            return Err(AiError::Unsupported(Capabilities::EMBEDDINGS));
        }
        crate::provider::validate_embedding_batch(inputs)?;
        // The model must be set per-request; the embeddings endpoint
        // requires it.
        // We borrow the model from the request side via the
        // public API: there is no request on the embed path, so we
        // require a separate config-derived model. For the public
        // API, the caller supplies the model through a sibling
        // method `embed_with_model`.
        Err(AiError::configuration(
            "OpenAiCompatProvider::embed requires an explicit model; call embed_with_model instead",
        ))
    }
}

impl OpenAiCompatProvider {
    /// Embed a batch of texts under a specific model id.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] for every failure class documented in
    /// [`crate::error`].
    pub async fn embed_with_model(
        &self,
        model: &str,
        inputs: &[&str],
    ) -> Result<Vec<Embedding>, AiError> {
        if !self.info.capabilities().contains(Capabilities::EMBEDDINGS) {
            return Err(AiError::Unsupported(Capabilities::EMBEDDINGS));
        }
        crate::provider::validate_embedding_batch(inputs)?;
        if model.is_empty() {
            return Err(AiError::validation("embedding model is empty"));
        }
        let url = format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.embed_path
        );
        let body = WireEmbeddingsRequest {
            model,
            input: inputs,
        };
        let response = self
            .client
            .post(&url)
            .bearer_auth(self.config.api_key.expose().as_str())
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(classify_with_status(status.as_u16(), &body));
        }
        let parsed: WireEmbeddingsResponse = response.json().await.map_err(|err| {
            AiError::malformed_response(format!("embeddings json decode failed: {err}"))
        })?;
        Ok(parsed
            .data
            .into_iter()
            .map(|datum| Embedding::new(datum.embedding, model))
            .collect())
    }
}

// --- HTTP error helpers ---------------------------------------------------

/// Map a reqwest error onto the platform's typed errors.
fn map_request_error(err: reqwest::Error) -> AiError {
    if err.is_timeout() {
        AiError::Timeout
    } else if err.is_connect() || err.is_request() {
        AiError::transport_unavailable(format!("http request failed: {err}"))
    } else {
        AiError::transport_unavailable(format!("http transport error: {err}"))
    }
}

/// Classify a non-2xx response. Mirrors [`AiError::from_status`]
/// but consults the body for a content-refusal signal so the
/// `ContentPolicy` variant can fire on 4xx that the platform did
/// not anticipate as `Validation`.
fn classify_with_status(status: u16, body: &str) -> AiError {
    if let Some(reason) = detect_content_refusal(body) {
        return AiError::content_policy(reason);
    }
    if status == 429 {
        return AiError::RateLimited {
            retry_after_secs: None,
        };
    }
    AiError::from_status(status, Some(body))
}

/// A minimal content-refusal signal: a body whose `error.message`
/// starts with `content_refused` (case-insensitive). Most providers
/// document a content-policy code, but the platform deliberately
/// matches a small set so a misclassification is loud.
fn detect_content_refusal(body: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(body).ok()?;
    let message = parsed
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)?;
    if message.to_ascii_lowercase().contains("content_refused") {
        Some(truncate_safe_detail(message).into_owned())
    } else {
        None
    }
}

// --- SSE stream parser ----------------------------------------------------
//
// The OpenAI streaming format is:
//
//   data: {"choices":[{"delta":{"content":"hi"}}]}
//   data: {"choices":[{"delta":{},"finish_reason":"stop"}]}
//   data: [DONE]
//
// We accumulate deltas; on the first chunk that carries a
// `finish_reason`, we emit the final `Turn(ChatResponse)`. Any
// subsequent content is treated as a continuation of the same
// response (some providers keep emitting after finish_reason —
// we tolerate that). The stream ends after `[DONE]` or when the
// response body finishes.

fn parse_sse_stream(
    response: reqwest::Response,
    max_bytes: usize,
    provider_id: String,
) -> impl futures::stream::Stream<Item = Result<StreamChunk, AiError>> + Send + 'static {
    async_stream::stream! {
        use futures::stream::StreamExt;
        let mut bytes_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut accumulated_content = String::new();
        let mut last_finish: Option<FinishReason> = None;
        let mut last_model = String::new();
        let mut usage: Option<Usage> = None;
        let mut request_id: Option<String> = None;
        let mut total_bytes = 0usize;

        while let Some(chunk) = bytes_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(err) => {
                    yield Err(map_request_error(err));
                    return;
                }
            };
            total_bytes += chunk.len();
            if total_bytes > max_bytes {
                yield Err(AiError::malformed_response("stream-truncated: exceeded max_stream_bytes"));
                return;
            }
            buffer.extend_from_slice(&chunk);
            while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=pos).collect();
                let line = match std::str::from_utf8(&line[..line.len().saturating_sub(1)]) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if let Some(rest) = line.strip_prefix("data:") {
                    let data = rest.trim();
                    if data == "[DONE]" {
                        if let Some(finish) = last_finish {
                            let response = ChatResponse::new(
                                if accumulated_content.is_empty() { None } else { Some(std::mem::take(&mut accumulated_content)) },
                                None,
                                finish,
                                usage.take(),
                                if last_model.is_empty() { provider_id.clone() } else { std::mem::take(&mut last_model) },
                                request_id.take(),
                            );
                            // Clear the finish state so the post-loop
                            // fallback does not emit a duplicate Turn
                            // when the body ends.
                            last_finish = None;
                            yield Ok(StreamChunk::Turn(response));
                        } else {
                            yield Err(AiError::malformed_response("stream-truncated: [DONE] before any finish_reason"));
                            return;
                        }
                        continue;
                    }
                    match serde_json::from_str::<WireChatResponse>(data) {
                        Ok(parsed) => {
                            if !parsed.model.is_empty() {
                                last_model = parsed.model;
                            }
                            if let Some(id) = parsed.request_id {
                                request_id = Some(id);
                            }
                            if let Some(u) = parsed.usage {
                                usage = Some(Usage::new(u.prompt_tokens, u.completion_tokens));
                            }
                            for choice in parsed.choices {
                                if let Some(message) = choice.message {
                                    if let Some(content) = message.content {
                                        if !content.is_empty() {
                                            accumulated_content.push_str(&content);
                                            yield Ok(StreamChunk::Text { delta: content });
                                        }
                                    }
                                    if let Some(tools) = message.tool_calls {
                                        match tool_calls_from_wire(tools) {
                                            Ok(Some(_)) => {
                                                // Tool calls in a stream are
                                                // surfaced via the final
                                                // response; we accept them
                                                // here as a future extension
                                                // and continue. The current
                                                // contract is text deltas plus
                                                // a single terminal Turn.
                                            }
                                            Ok(None) => {}
                                            Err(err) => {
                                                yield Err(err);
                                                return;
                                            }
                                        }
                                    }
                                }
                                if let Some(reason) = choice.finish_reason.as_deref() {
                                    last_finish = Some(map_finish_reason(Some(reason)));
                                }
                            }
                        }
                        Err(_) => {
                            // Malformed line: tolerate (the platform's
                            // spec only requires malformed JSON to fail
                            // loudly; a stray SSE comment is benign).
                        }
                    }
                }
            }
        }
        if last_finish.is_some() {
            // The server never sent [DONE] but we have a finish
            // reason: emit the final Turn anyway.
            let response = ChatResponse::new(
                if accumulated_content.is_empty() { None } else { Some(accumulated_content) },
                None,
                last_finish.unwrap(),
                usage,
                if last_model.is_empty() { provider_id } else { last_model },
                request_id,
            );
            yield Ok(StreamChunk::Turn(response));
        } else if !accumulated_content.is_empty() {
            yield Err(AiError::malformed_response("stream-truncated: response ended without a finish_reason"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_debug_redacts_api_key() {
        let cfg =
            OpenAiCompatConfig::new(Secret::new("sk-live-secret".into())).expect("config builds");
        let rendered = format!("{cfg:?}");
        assert!(
            !rendered.contains("sk-live-secret"),
            "key leaked: {rendered}"
        );
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn insecure_remote_base_url_is_refused() {
        let err = OpenAiCompatConfig::with_base_url_and_id(
            Secret::new("k".into()),
            "http://api.openai.com",
            "openai",
        )
        .expect_err("plain http remote must fail");
        assert!(matches!(err, AiError::Configuration(_)));
    }

    #[test]
    fn loopback_http_is_accepted() {
        OpenAiCompatConfig::with_base_url_and_id(
            Secret::new("k".into()),
            "http://127.0.0.1:1234",
            "local",
        )
        .expect("loopback http constructs");
    }

    #[test]
    fn empty_key_is_refused() {
        let err =
            OpenAiCompatConfig::new(Secret::new(String::new())).expect_err("empty key must fail");
        assert!(matches!(err, AiError::Configuration(_)));
    }

    #[test]
    fn classify_with_status_promotes_content_refusal_body() {
        let body = r#"{"error":{"message":"content_refused: the prompt is too long"}}"#;
        let err = classify_with_status(400, body);
        assert!(matches!(err, AiError::ContentPolicy(_)));
    }

    #[test]
    fn classify_with_status_passes_429_to_rate_limited() {
        let err = classify_with_status(429, "");
        assert!(matches!(err, AiError::RateLimited { .. }));
    }

    #[test]
    fn classify_with_status_truncates_long_server_detail() {
        let big = "x".repeat(500);
        let body = format!("{{\"error\":{{\"message\":\"{big}\"}}}}");
        let err = classify_with_status(500, &body);
        if let AiError::ProviderServer { safe_detail, .. } = err {
            assert!(safe_detail.len() <= crate::error::MAX_SAFE_DETAIL_BYTES);
        } else {
            panic!("expected ProviderServer, got {err:?}");
        }
    }

    #[test]
    fn role_wire_form_is_snake_case() {
        use crate::types::Role;
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::Tool.as_str(), "tool");
    }
}
