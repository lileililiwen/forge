//! Provider-neutral AI capability contract.
//!
//! `platform-ai` is a satellite crate, modelled on
//! [`platform_mailing`](platform-mailing). It depends on
//! [`platform_core`](platform_core) (error categories, clocks, correlation
//! IDs, redaction) and exposes one typed, provider-neutral contract plus
//! three implementations:
//!
//! * [`AiProvider`] / [`EmbeddingProvider`] — the async traits every
//!   consumer programs against; default trait methods return
//!   [`AiError::Unsupported`] for capabilities absent from the
//!   provider's declared [`ProviderInfo`].
//! * [`mock::MockAi`] (feature `mock`) — scripted, network-free
//!   outcomes for consumer test suites. It lives in this crate so
//!   production crates never touch the test-only
//!   `platform-testing` crate.
//! * [`openai_compat::OpenAiCompatProvider`] (feature `openai-compat`)
//!   — the OpenAI-compatible HTTP adapter, configurable by
//!   `base_url` so a single implementation covers OpenAI, DeepSeek,
//!   Zhipu, and any other vendor that exposes the same wire shape.
//! * [`ollama::OllamaProvider`] (feature `ollama`) — the local-ollama
//!   adapter over `/api/chat` and `/api/embed`.
//!
//! Validation always runs before any network call, and every
//! [`AiError`] maps onto an existing
//! [`platform_core::ErrorCategory`] without leaking API keys, prompts,
//! completions, or raw response bodies.

#![forbid(unsafe_code)]

pub mod error;
pub mod provider;
pub mod types;

#[cfg(feature = "mock")]
pub mod mock;

#[cfg(feature = "openai-compat")]
pub mod openai_compat;

#[cfg(feature = "ollama")]
pub mod ollama;

pub use error::AiError;
pub use provider::{AiContext, AiProvider, EmbeddingProvider};
pub use types::{
    Capabilities, ChatRequest, ChatResponse, Content, Embedding, FinishReason, GenerationOptions,
    Message, ProviderInfo, ProviderKind, Role, StreamChunk, ToolCall, ToolDefinition, Usage,
};
