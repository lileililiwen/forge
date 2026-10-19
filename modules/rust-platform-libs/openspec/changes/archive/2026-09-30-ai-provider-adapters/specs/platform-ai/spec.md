# platform-ai Specification

## ADDED Requirements

### Requirement: A provider-neutral AI chat contract SHALL exist (AI-001)

The `platform-ai` crate MUST expose `Message`, `ChatRequest`,
`GenerationOptions`, `ChatResponse`, `ToolCall`, `Usage`, `AiError`, and the
`AiProvider` trait, validating every request against its declared bounds
before any network call, and every `AiError` MUST map onto an existing
`platform-core` `ErrorCategory` without leaking API keys, prompts,
completions, or raw response bodies.

#### Scenario: Valid request served by injected provider

- **WHEN** a consumer sends a valid `ChatRequest` through any `AiProvider`
  implementation
- **THEN** the call returns a `ChatResponse` with finish reason and, when
  the provider reported them, token `Usage` and model id

#### Scenario: Invalid request never reaches the network

- **WHEN** the request has zero or more than 256 messages, a temperature
  outside 0.0–2.0, more than 4 stop sequences, or an empty embedding batch
- **THEN** the call returns `AiError::Validation` and no HTTP request is
  issued

#### Scenario: Error text is safe

- **WHEN** any `AiError` produced from a failure containing a secret key or
  prompt material is rendered with `Display`
- **THEN** the output contains neither the key nor prompt or body content
  beyond a redacted, bounded server message

### Requirement: Providers SHALL declare capabilities and refuse others (AI-002)

`AiProvider::info` MUST return a `ProviderInfo` with a `Capabilities` set,
and the default trait implementations of `stream`,
`generate_structured`, and `EmbeddingProvider::embed` for undeclared
capabilities MUST return `AiError::Unsupported` naming the capability.

#### Scenario: Declared capability executes

- **WHEN** a provider declares `STREAMING` and `stream` is called
- **THEN** the stream yields `Text` chunks and exactly one final `Turn`

#### Scenario: Undeclared capability refused

- **WHEN** a provider does not declare `EMBEDDINGS` and embedding is
  attempted through it
- **THEN** the call returns `AiError::Unsupported` and no request is sent

### Requirement: An OpenAI-compatible adapter SHALL unify configurable vendors (AI-003)

Under the `openai-compat` cargo feature, the crate MUST provide a provider
speaking `{base_url}{chat_path}` chat/completions and
`{base_url}{embed_path}` embeddings with bearer `Secret` auth, supporting
streaming, tool calls, and JSON-schema structured output; plain HTTP MUST be
accepted only for loopback hosts.

#### Scenario: Vendor reached by base_url change

- **WHEN** the adapter is configured with OpenAI, DeepSeek, or Zhipu
  base_url/path values against canned fixtures
- **THEN** chat, tools, structured, embeddings, and streaming all succeed
  through the single adapter

#### Scenario: Provider failure maps by status

- **WHEN** a fixture server answers 401, 429 with `Retry-After`, 5xx, or
  malformed JSON
- **THEN** the adapter returns `Unauthorized`, `RateLimited(seconds)`,
  `ProviderServer(status)`, and `MalformedResponse` respectively, with no
  automatic retry

#### Scenario: Insecure remote refused

- **WHEN** `OpenAiCompatConfig` sets a non-loopback plain-http base_url
- **THEN** construction fails with `AiError::Configuration`

### Requirement: An ollama adapter SHALL serve local models (AI-004)

Under the `ollama` cargo feature, the crate MUST provide a provider using
native `/api/chat` (NDJSON streaming, `"format"` schema for structured
output) and `/api/embed`, defaulting to the loopback endpoint with no
credential material in its configuration.

#### Scenario: Local chat completes

- **WHEN** a chat request is served by a canned ollama NDJSON fixture
- **THEN** the response or stream parses into the contract types with a
  final `Turn`

#### Scenario: Unreachable daemon

- **WHEN** the configured ollama host refuses connection
- **THEN** the call returns `AiError::TransportUnavailable` without retry

### Requirement: A deterministic mock provider SHALL support consumer tests (AI-005)

The `mock` cargo feature MUST provide a scriptable in-crate `AiProvider`
(and embedding source) returning configured success, rate-limit, timeout,
and truncation outcomes with injected-clock timestamps and no network, so
production crates never reference `platform-testing`.

#### Scenario: Scripted outcomes

- **WHEN** a test scripts the mock to succeed then to return `RateLimited`
- **THEN** the two calls produce a `ChatResponse` and the declared error
  respectively

### Requirement: AI transport dependencies SHALL stay quarantined by the satellite allowlist (AI-006)

`platform-arch-tests` MUST allow `reqwest`, `tokio`, and `futures` to be
reachable ONLY from `platform-ai` among production crates, keeping the four
foundation crates, `platform-mailing`, and the `platform-testing`
prohibition unchanged.

#### Scenario: Compliant tree passes

- **WHEN** the workspace graph is walked with `platform-ai` as the only
  reqwest consumer
- **THEN** the architecture tests pass

#### Scenario: Foundation violation fails

- **WHEN** the rule function evaluates a synthesized graph attaching
  `reqwest` to any foundation crate
- **THEN** it reports a violation naming the crate and dependency

#### Scenario: Unknown satellite denied

- **WHEN** a crate absent from the allowlist table reaches `reqwest`
- **THEN** the architecture tests fail until an explicit policy change
  names it
