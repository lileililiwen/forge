//! Provider-neutral chat, structured, embedding, and stream types.
//!
//! Validation limits (all enforced in [`ChatRequest::validate`],
//! [`GenerationOptions::validate`], and [`AiError::Validation`](crate::AiError)
//! returns from every transport call before any socket is opened):
//!
//! * messages: 1..=256 per chat request;
//! * temperature: 0.0..=2.0;
//! * `max_output_tokens`: 1..=32 768;
//! * stop sequences: at most 4;
//! * embedding batch: 1..=64 texts;
//! * model id: at most 128 characters (bounded on the request side;
//!   the response-side `model` is the as-reported provider string and
//!   is not bounded here).

use crate::error::AiError;

/// Maximum number of messages in a single chat request.
pub const MAX_MESSAGES: usize = 256;

/// Maximum number of stop sequences in a single chat request.
pub const MAX_STOP_SEQUENCES: usize = 4;

/// Maximum number of texts in a single embedding batch.
pub const MAX_EMBED_TEXTS: usize = 64;

/// Maximum number of `max_output_tokens` allowed in a request.
pub const MAX_OUTPUT_TOKENS: u32 = 32_768;

/// Maximum length of a model identifier on the request side.
pub const MAX_MODEL_LEN: usize = 128;

/// Maximum length of a tool name on the request side.
pub const MAX_TOOL_NAME_LEN: usize = 64;

/// Maximum length of a tool call id or stop sequence the platform will
/// forward verbatim.
pub const MAX_TOOL_ID_LEN: usize = 128;

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// System prompt.
    System,
    /// User turn.
    User,
    /// Assistant turn.
    Assistant,
    /// Tool result carrier; the platform treats it like any other role
    /// — its content is forwarded verbatim.
    Tool,
}

impl Role {
    /// Stable wire form (`snake_case`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

/// Text content of a message. Multimodal content parts are deferred
/// (design L8-b) — the platform carries text only today.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Content {
    text: String,
}

impl Content {
    /// Build a text content payload.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self { text: value.into() }
    }

    /// Borrow the inner text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Consume and return the inner text.
    #[must_use]
    pub fn into_string(self) -> String {
        self.text
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::text(value)
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self { text: value }
    }
}

/// One message in a chat request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    role: Role,
    content: Content,
    /// Set on `Role::Tool` results; the platform forwards it verbatim.
    tool_call_id: Option<String>,
}

impl Message {
    /// Build a message.
    #[must_use]
    pub fn new(role: Role, content: impl Into<Content>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Build a `Role::Tool` result message carrying a `tool_call_id`.
    #[must_use]
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<Content>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// Borrow the role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// Borrow the content.
    #[must_use]
    pub const fn content(&self) -> &Content {
        &self.content
    }

    /// Borrow the tool call id, if any.
    #[must_use]
    pub fn tool_call_id(&self) -> Option<&str> {
        self.tool_call_id.as_deref()
    }
}

/// Generation controls. Every field is optional; the request is
/// validated at adapter entry against the declared bounds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GenerationOptions {
    temperature: Option<f32>,
    max_output_tokens: Option<u32>,
    stop: Vec<String>,
    tools: Option<Vec<ToolDefinition>>,
    seed: Option<u64>,
}

impl GenerationOptions {
    /// Set the sampling temperature. The bound 0.0..=2.0 is enforced
    /// by [`GenerationOptions::validate`].
    #[must_use]
    pub fn with_temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    /// Set the maximum output tokens. The bound 1..=32 768 is
    /// enforced by [`GenerationOptions::validate`].
    #[must_use]
    pub fn with_max_output_tokens(mut self, value: u32) -> Self {
        self.max_output_tokens = Some(value);
        self
    }

    /// Add a stop sequence.
    #[must_use]
    pub fn with_stop(mut self, value: impl Into<String>) -> Self {
        self.stop.push(value.into());
        self
    }

    /// Replace the tool definitions.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set a seed for deterministic sampling, when the provider
    /// supports it.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Borrow the temperature, if set.
    #[must_use]
    pub const fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Borrow the max-output-tokens, if set.
    #[must_use]
    pub const fn max_output_tokens(&self) -> Option<u32> {
        self.max_output_tokens
    }

    /// Borrow the stop sequences.
    #[must_use]
    pub fn stop(&self) -> &[String] {
        &self.stop
    }

    /// Borrow the tool definitions, if any.
    #[must_use]
    pub fn tools(&self) -> Option<&[ToolDefinition]> {
        self.tools.as_deref()
    }

    /// Borrow the seed, if set.
    #[must_use]
    pub const fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Enforce every bound. Called by every transport before opening
    /// a socket.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for out-of-range `temperature`,
    /// `max_output_tokens`, or for more than
    /// [`MAX_STOP_SEQUENCES`] stop sequences.
    pub fn validate(&self) -> Result<(), AiError> {
        if let Some(t) = self.temperature {
            if !t.is_finite() || !(0.0..=2.0).contains(&t) {
                return Err(AiError::Validation(format!(
                    "temperature {t} is outside the 0.0..=2.0 range"
                )));
            }
        }
        if let Some(max) = self.max_output_tokens {
            if max == 0 || max > MAX_OUTPUT_TOKENS {
                return Err(AiError::Validation(format!(
                    "max_output_tokens {max} is outside 1..={MAX_OUTPUT_TOKENS}"
                )));
            }
        }
        if self.stop.len() > MAX_STOP_SEQUENCES {
            return Err(AiError::Validation(format!(
                "stop sequences count {} exceeds the {MAX_STOP_SEQUENCES} limit",
                self.stop.len()
            )));
        }
        Ok(())
    }
}

/// A tool description forwarded to providers that support tool calls.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDefinition {
    name: String,
    description: Option<String>,
    parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Build a tool definition.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for an empty or over-long
    /// `name` or for `parameters` that is not a JSON object.
    pub fn new(
        name: impl Into<String>,
        description: Option<String>,
        parameters: serde_json::Value,
    ) -> Result<Self, AiError> {
        let name = name.into();
        if name.is_empty() {
            return Err(AiError::Validation("tool name is empty".into()));
        }
        if name.len() > MAX_TOOL_NAME_LEN {
            return Err(AiError::Validation(format!(
                "tool name length {} exceeds {MAX_TOOL_NAME_LEN}",
                name.len()
            )));
        }
        if !parameters.is_object() {
            return Err(AiError::Validation(
                "tool parameters must be a JSON object".into(),
            ));
        }
        Ok(Self {
            name,
            description,
            parameters,
        })
    }

    /// Borrow the tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the tool description, if any.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Borrow the JSON-Schema parameters.
    #[must_use]
    pub fn parameters(&self) -> &serde_json::Value {
        &self.parameters
    }
}

/// A tool invocation the model asked the caller to perform.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

impl ToolCall {
    /// Build a tool call.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for an empty or over-long id,
    /// an empty or over-long name, or arguments that are not a JSON
    /// object.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Result<Self, AiError> {
        let id = id.into();
        let name = name.into();
        if id.is_empty() {
            return Err(AiError::Validation("tool call id is empty".into()));
        }
        if id.len() > MAX_TOOL_ID_LEN {
            return Err(AiError::Validation(format!(
                "tool call id length {} exceeds {MAX_TOOL_ID_LEN}",
                id.len()
            )));
        }
        if name.is_empty() {
            return Err(AiError::Validation("tool call name is empty".into()));
        }
        if name.len() > MAX_TOOL_NAME_LEN {
            return Err(AiError::Validation(format!(
                "tool call name length {} exceeds {MAX_TOOL_NAME_LEN}",
                name.len()
            )));
        }
        if !arguments.is_object() {
            return Err(AiError::Validation(
                "tool call arguments must be a JSON object".into(),
            ));
        }
        Ok(Self {
            id,
            name,
            arguments,
        })
    }

    /// Borrow the call id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Borrow the tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the parsed arguments.
    #[must_use]
    pub fn arguments(&self) -> &serde_json::Value {
        &self.arguments
    }
}

/// Why the model stopped emitting tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinishReason {
    /// The model emitted an end-of-turn token naturally.
    Stop,
    /// The model wants the caller to invoke a tool.
    ToolCalls,
    /// The model hit `max_output_tokens` or the provider's hard cap.
    Length,
    /// The provider's content policy refused the response.
    ContentRefused,
    /// The provider could not classify the reason.
    Unknown,
}

impl FinishReason {
    /// Stable wire form (`snake_case`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::ToolCalls => "tool_calls",
            Self::Length => "length",
            Self::ContentRefused => "content_refused",
            Self::Unknown => "unknown",
        }
    }
}

/// Provider-reported token usage. The platform NEVER estimates these
/// locally (design L4); the values are passed through verbatim when
/// the provider reports them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

impl Usage {
    /// Build a usage payload.
    #[must_use]
    pub const fn new(input_tokens: Option<u64>, output_tokens: Option<u64>) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Borrow the input token count, if reported.
    #[must_use]
    pub const fn input_tokens(&self) -> Option<u64> {
        self.input_tokens
    }

    /// Borrow the output token count, if reported.
    #[must_use]
    pub const fn output_tokens(&self) -> Option<u64> {
        self.output_tokens
    }
}

/// A validated-or-validatable chat request.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatRequest {
    model: Option<String>,
    messages: Vec<Message>,
    options: GenerationOptions,
}

impl ChatRequest {
    /// Start building a request with a single message.
    #[must_use]
    pub fn builder(message: Message) -> ChatRequestBuilder {
        ChatRequestBuilder {
            model: None,
            messages: vec![message],
            options: GenerationOptions::default(),
        }
    }

    /// Borrow the model, if set.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Borrow the messages.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Borrow the generation options.
    #[must_use]
    pub const fn options(&self) -> &GenerationOptions {
        &self.options
    }

    /// Enforce every bound. Called by every transport before opening
    /// a socket.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for an empty or over-long
    /// model id, an empty or over-cap messages list, an empty tool
    /// definition in `options.tools`, or any of the
    /// [`GenerationOptions`] bounds.
    pub fn validate(&self) -> Result<(), AiError> {
        if let Some(model) = &self.model {
            if model.is_empty() {
                return Err(AiError::Validation("model is empty".into()));
            }
            if model.len() > MAX_MODEL_LEN {
                return Err(AiError::Validation(format!(
                    "model length {} exceeds {MAX_MODEL_LEN}",
                    model.len()
                )));
            }
        }
        if self.messages.is_empty() {
            return Err(AiError::Validation("chat request has no messages".into()));
        }
        if self.messages.len() > MAX_MESSAGES {
            return Err(AiError::Validation(format!(
                "chat request has {} messages; the limit is {MAX_MESSAGES}",
                self.messages.len()
            )));
        }
        self.options.validate()?;
        if let Some(tools) = self.options.tools() {
            for tool in tools {
                // ToolDefinition::new already enforces the bounds; the
                // call here is a defense in depth.
                ToolDefinition::new(
                    tool.name.clone(),
                    tool.description.clone(),
                    tool.parameters.clone(),
                )?;
            }
        }
        Ok(())
    }
}

/// Fluent builder for [`ChatRequest`]; [`build`](Self::build)
/// enforces every bound.
#[derive(Debug, Clone)]
pub struct ChatRequestBuilder {
    model: Option<String>,
    messages: Vec<Message>,
    options: GenerationOptions,
}

impl ChatRequestBuilder {
    /// Override the model id.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a message.
    #[must_use]
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Replace the generation options wholesale.
    #[must_use]
    pub fn options(mut self, options: GenerationOptions) -> Self {
        self.options = options;
        self
    }

    /// Borrow the messages collected so far.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Build the request, enforcing every bound.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Validation`] for the same reasons as
    /// [`ChatRequest::validate`].
    pub fn build(self) -> Result<ChatRequest, AiError> {
        let request = ChatRequest {
            model: self.model,
            messages: self.messages,
            options: self.options,
        };
        request.validate()?;
        Ok(request)
    }
}

/// A single non-streaming chat response.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatResponse {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    finish: FinishReason,
    usage: Option<Usage>,
    model: String,
    provider_request_id: Option<String>,
}

impl ChatResponse {
    /// Build a response.
    #[must_use]
    pub fn new(
        content: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
        finish: FinishReason,
        usage: Option<Usage>,
        model: impl Into<String>,
        provider_request_id: Option<String>,
    ) -> Self {
        Self {
            content,
            tool_calls,
            finish,
            usage,
            model: model.into(),
            provider_request_id,
        }
    }

    /// Borrow the textual content, if any.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Borrow the tool calls, if any.
    #[must_use]
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.tool_calls.as_deref()
    }

    /// Borrow the finish reason.
    #[must_use]
    pub const fn finish(&self) -> FinishReason {
        self.finish
    }

    /// Borrow the usage, if reported.
    #[must_use]
    pub const fn usage(&self) -> Option<Usage> {
        self.usage
    }

    /// Borrow the as-reported model id.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Borrow the provider's request id, if surfaced.
    #[must_use]
    pub fn provider_request_id(&self) -> Option<&str> {
        self.provider_request_id.as_deref()
    }
}

/// A chunk of a streaming chat response.
///
/// The contract is: zero or more `Text` deltas, then exactly one
/// `Turn(ChatResponse)` finalising the response. An error mid-stream
/// terminates the stream and is the last value the consumer sees.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamChunk {
    /// A text delta. The consumer appends `delta` to its accumulated
    /// text.
    Text {
        /// The text delta.
        delta: String,
    },
    /// The terminal chunk: the full final [`ChatResponse`].
    Turn(ChatResponse),
}

impl StreamChunk {
    /// Borrow the text delta if this chunk is `Text`.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text { delta } => Some(delta),
            Self::Turn(_) => None,
        }
    }

    /// Borrow the final response if this chunk is `Turn`.
    #[must_use]
    pub fn turn(&self) -> Option<&ChatResponse> {
        match self {
            Self::Text { .. } => None,
            Self::Turn(turn) => Some(turn),
        }
    }
}

/// A single embedding vector together with the as-reported model id.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    values: Vec<f32>,
    model: String,
}

impl Embedding {
    /// Build an embedding.
    #[must_use]
    pub fn new(values: Vec<f32>, model: impl Into<String>) -> Self {
        Self {
            values,
            model: model.into(),
        }
    }

    /// Borrow the vector.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Borrow the as-reported model id.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Capability bits declared by a provider through
/// [`ProviderInfo::capabilities`].
///
/// This is a hand-rolled bit-set on `u8` rather than the `bitflags`
/// crate, to keep `platform-ai` free of a new runtime dependency
/// (the rest of the foundation prefers zero-dep primitives). The
/// public API is intentionally tiny: `empty`, `all`, `is_empty`,
/// `contains`, `insert`, `remove`, `iter`, the `|`, `&=`, and `-`
/// operators, and `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Capabilities(u8);

impl Capabilities {
    /// Provider supports [`AiProvider::stream`](crate::provider::AiProvider::stream).
    pub const STREAMING: Self = Self(0b0000_0001);
    /// Provider accepts [`ToolDefinition`]s and emits
    /// [`ToolCall`]s.
    pub const TOOLS: Self = Self(0b0000_0010);
    /// Provider accepts a JSON-schema response format.
    pub const STRUCTURED: Self = Self(0b0000_0100);
    /// Provider implements [`EmbeddingProvider`](crate::provider::EmbeddingProvider).
    pub const EMBEDDINGS: Self = Self(0b0000_1000);

    /// The empty capability set: the provider supports no optional
    /// capabilities.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// The set of every declared capability bit.
    #[must_use]
    pub const fn all_bits() -> Self {
        Self(Self::STREAMING.0 | Self::TOOLS.0 | Self::STRUCTURED.0 | Self::EMBEDDINGS.0)
    }

    /// True when no capability is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True when every bit of `other` is set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Return a new set with `other`'s bits added.
    #[must_use]
    pub const fn insert(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return a new set with `other`'s bits removed.
    #[must_use]
    pub const fn remove(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Iterate the individual set bits.
    #[must_use]
    pub fn iter(self) -> CapabilitiesIter {
        CapabilitiesIter { remaining: self.0 }
    }
}

impl std::ops::BitOr for Capabilities {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Capabilities {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for Capabilities {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::Sub for Capabilities {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 & !rhs.0)
    }
}

/// Iterator over the individual set bits of a [`Capabilities`].
#[derive(Debug, Clone)]
pub struct CapabilitiesIter {
    remaining: u8,
}

impl Iterator for CapabilitiesIter {
    type Item = Capabilities;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let bit = 1u8 << self.remaining.trailing_zeros();
        self.remaining &= !bit;
        Some(Capabilities(bit))
    }
}

impl std::fmt::Display for Capabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for cap in self.iter() {
            if !first {
                f.write_str("|")?;
            }
            first = false;
            match cap {
                Self::STREAMING => f.write_str("STREAMING")?,
                Self::TOOLS => f.write_str("TOOLS")?,
                Self::STRUCTURED => f.write_str("STRUCTURED")?,
                Self::EMBEDDINGS => f.write_str("EMBEDDINGS")?,
                _ => write!(f, "UNKNOWN({:02x})", cap.0)?,
            }
        }
        if first {
            f.write_str("NONE")?;
        }
        Ok(())
    }
}

/// Stable kind label for a provider. Used in logs and error context;
/// not a wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    /// OpenAI-compatible HTTP adapter.
    OpenAiCompat,
    /// Native ollama adapter.
    Ollama,
    /// Deterministic in-crate mock.
    Mock,
    /// Application-owned provider not classified by the platform.
    Custom,
}

impl ProviderKind {
    /// Stable wire form (`snake_case`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompat => "openai_compat",
            Self::Ollama => "ollama",
            Self::Mock => "mock",
            Self::Custom => "custom",
        }
    }
}

/// Provider self-description. Returned by [`AiProvider::info`] and
/// consulted by the trait default methods to decide whether a
/// capability is supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    provider_id: String,
    kind: ProviderKind,
    capabilities: Capabilities,
}

impl ProviderInfo {
    /// Build a provider info record.
    #[must_use]
    pub fn new(
        provider_id: impl Into<String>,
        kind: ProviderKind,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            kind,
            capabilities,
        }
    }

    /// Borrow the provider id (e.g. `"openai"`, `"ollama"`,
    /// `"mock"`).
    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Borrow the kind.
    #[must_use]
    pub const fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// Borrow the capability set.
    #[must_use]
    pub const fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> Message {
        Message::new(Role::User, Content::text(text))
    }

    #[test]
    fn chat_request_builder_enforces_message_count_bounds() {
        let mut builder = ChatRequest::builder(user("hi"));
        for i in 0..(MAX_MESSAGES - 1) {
            builder = builder.message(user(&format!("turn {i}")));
        }
        assert_eq!(builder.messages().len(), MAX_MESSAGES);
        assert!(builder.clone().build().is_ok(), "256 messages is allowed");

        // 257 messages overflows.
        let overflow = builder.message(user("extra")).build();
        assert!(overflow.is_err(), "257 messages must fail");

        // Empty messages is refused.
        let empty = ChatRequestBuilder {
            model: None,
            messages: vec![],
            options: GenerationOptions::default(),
        }
        .build();
        assert!(matches!(empty, Err(AiError::Validation(_))));
    }

    #[test]
    fn chat_request_enforces_model_id_length() {
        let too_long = "m".repeat(MAX_MODEL_LEN + 1);
        let request = ChatRequest::builder(user("hi")).model(too_long).build();
        assert!(matches!(request, Err(AiError::Validation(_))));

        let exactly = "m".repeat(MAX_MODEL_LEN);
        let request = ChatRequest::builder(user("hi")).model(exactly).build();
        assert!(request.is_ok());
    }

    #[test]
    fn generation_options_enforce_temperature_and_tokens_bounds() {
        let ok = GenerationOptions::default()
            .with_temperature(0.7)
            .with_max_output_tokens(1_000)
            .with_stop("END")
            .validate();
        assert!(ok.is_ok());

        // NaN temperature is refused (not finite).
        let bad = GenerationOptions::default()
            .with_temperature(f32::NAN)
            .validate();
        assert!(matches!(bad, Err(AiError::Validation(_))));

        // Out-of-range temperature is refused.
        let bad = GenerationOptions::default()
            .with_temperature(2.1)
            .validate();
        assert!(matches!(bad, Err(AiError::Validation(_))));

        // 0 max_output_tokens is refused.
        let bad = GenerationOptions::default()
            .with_max_output_tokens(0)
            .validate();
        assert!(matches!(bad, Err(AiError::Validation(_))));

        // Over-cap max_output_tokens is refused.
        let bad = GenerationOptions::default()
            .with_max_output_tokens(MAX_OUTPUT_TOKENS + 1)
            .validate();
        assert!(matches!(bad, Err(AiError::Validation(_))));

        // 5 stop sequences are refused; 4 are allowed.
        let five = GenerationOptions::default()
            .with_stop("a")
            .with_stop("b")
            .with_stop("c")
            .with_stop("d")
            .with_stop("e")
            .validate();
        assert!(matches!(five, Err(AiError::Validation(_))));
        let four = GenerationOptions::default()
            .with_stop("a")
            .with_stop("b")
            .with_stop("c")
            .with_stop("d")
            .validate();
        assert!(four.is_ok());
    }

    #[test]
    fn tool_definition_and_call_enforce_shape() {
        let ok = ToolDefinition::new(
            "get_weather",
            Some("Fetch the weather".into()),
            serde_json::json!({"type": "object"}),
        );
        assert!(ok.is_ok());

        let bad = ToolDefinition::new("", None, serde_json::json!({}));
        assert!(matches!(bad, Err(AiError::Validation(_))));

        let bad = ToolDefinition::new("ok", None, serde_json::json!([]));
        assert!(matches!(bad, Err(AiError::Validation(_))));

        let bad = ToolCall::new("", "ok", serde_json::json!({}));
        assert!(matches!(bad, Err(AiError::Validation(_))));

        let bad = ToolCall::new("id", "ok", serde_json::json!("not object"));
        assert!(matches!(bad, Err(AiError::Validation(_))));

        let ok = ToolCall::new(
            "call_1",
            "get_weather",
            serde_json::json!({"city": "Berlin"}),
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn capabilities_round_trip_through_provider_info() {
        let info = ProviderInfo::new(
            "openai",
            ProviderKind::OpenAiCompat,
            Capabilities::STREAMING | Capabilities::TOOLS,
        );
        assert!(info.capabilities().contains(Capabilities::STREAMING));
        assert!(!info.capabilities().contains(Capabilities::EMBEDDINGS));
        assert_eq!(info.kind(), ProviderKind::OpenAiCompat);
        assert_eq!(info.provider_id(), "openai");
    }

    #[test]
    fn stream_chunk_exposes_text_and_turn() {
        let text = StreamChunk::Text { delta: "hi".into() };
        assert_eq!(text.text(), Some("hi"));
        assert!(text.turn().is_none());

        let turn = StreamChunk::Turn(ChatResponse::new(
            Some("hi".into()),
            None,
            FinishReason::Stop,
            None,
            "gpt-4o",
            None,
        ));
        assert!(turn.text().is_none());
        assert!(turn.turn().is_some());
    }
}
