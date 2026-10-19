# Design: platform-ai crate and satellite budget extension

## 1. Implementation boundary

- Repository: `rust-platform-libs` only.
- New crate: `crates/platform-ai/` — `Cargo.toml`, `src/lib.rs`,
  `src/types.rs` (messages/request/response/tool/usage), `src/error.rs`,
  `src/provider.rs` (`AiProvider`, `EmbeddingProvider`, `ProviderInfo`,
  `Capabilities`), `src/mock.rs`, `src/openai_compat.rs`, `src/ollama.rs`,
  `tests/ai_types.rs`, `tests/ai_mock.rs`, `tests/ai_openai_compat.rs`,
  `tests/ai_ollama.rs`.
- Changed files: workspace `Cargo.toml` (member + `reqwest`, `futures`,
  `wiremock` (dev) entries), `crates/platform-arch-tests/` (allowlist table
  row for `platform-ai` with budget `{reqwest, tokio, futures}`),
  `README.md`, `ROADMAP.md`, `HANDOFF.md` at completion.
- Must not change: the four foundation crates, `platform-mailing`,
  `platform-testing`, any sibling repository, CI semantics.

## 2. Language and runtime

- Rust, edition 2021, workspace `rust-version` (as pinned by
  `rust-platform-consumer-baseline`).
- `async-trait` on provider traits (dyn-injectable, mailing precedent).
- Streaming: `futures::stream::BoxStream<'static, Result<StreamChunk,
  AiError>>`; reqwest `stream` feature for SSE/NDJSON reading.
- HTTP: `reqwest = { version = "0.12", default-features = false, features =
  ["json", "stream", "rustls-tls", "http2"] }` — rustls-only per the
  `platform-mailing` TLS ledger entry; no native-tls edge anywhere.
- Timestamps/ids: `time`, `uuid` (both existing workspace deps).
- JSON: `serde`, `serde_json` (workspace).
- Errors: hand-rolled `Display` implementations onto
  `platform_core::{ErrorCategory, SafeError}` — following the
  `platform-mailing` precedent (`thiserror` is a workspace dep but the
  satellite convention is explicit mapping, ledger L7).
- Commands: `cargo test --workspace`; `cargo test -p platform-ai`;
  `cargo test -p platform-ai --no-default-features`;
  `cargo test -p platform-ai --features openai-compat,ollama,mock`;
  `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`.

## 3. Ownership and shared code

- Decision from reconnaissance: **extend shared owner** — recurring seam in
  two consumers; satellite pattern already approved in B1.
- Dependency direction: consumers → `platform-ai` → `platform-core`
  (+ reqwest/tokio/futures under features). Foundation crates never depend
  on `platform-ai`; `platform-ai` never depends on `platform-testing` (mock
  is an in-crate `mock` feature, mail precedent).
- Allowlist growth is the single table row in `platform-arch-tests`; the
  rule function itself is unchanged (B1 design §7).
- Version/release: workspace version; `publish = false` retained — the
  `registry-publication` change (roadmap D4) owns flipping it, not this one.

## 4. Behavioral model

| Element | Definition |
|---|---|
| `Role` | `System`, `User`, `Assistant`, `Tool` (result carrier) |
| `Message` | role + `Content` (text; multimodal deferred, ledger L8-b) + optional `tool_call_id` for `Tool` role |
| `ChatRequest` | `model: Option<bounded String>`, messages (1..=256), `GenerationOptions` |
| `GenerationOptions` | `temperature` (0.0–2.0, default 0.7), `max_output_tokens` (1..=32768), `stop: Vec<bounded>` (≤ 4), `tools: Option<Vec<ToolDefinition>>`, `seed: Option<u64>` |
| `ToolDefinition` / `ToolCall` | name (bounded), description, JSON-Schema `parameters: serde_json::Value` / id + name + `arguments: Value` |
| `ChatResponse` | `content: Option<String>`, `tool_calls: Option<Vec<ToolCall>>`, `finish: FinishReason` (`Stop`, `ToolCalls`, `Length`, `ContentRefused`, `Unknown`), `usage: Option<Usage>`, `model: String` (as-reported), `provider_request_id: Option<bounded String>` |
| `StreamChunk` | `Text(String)` deltas then exactly one final `Turn(ChatResponse)`; error mid-stream terminates the stream |
| `Usage` | input/output token counts as reported by the provider; NEVER estimated locally (L4) |
| `Embedding` | `Vec<f32>` + model id; batch input `&[&str]` (1..=64 texts) |
| `ProviderInfo` | `provider_id`, `kind` (`OpenAiCompat`/`Ollama`/`Mock`/`Custom`), `Capabilities` bit-set {STREAMING, TOOLS, STRUCTURED, EMBEDDINGS} |
| `AiContext` | `CorrelationId` + `&dyn Clock` (platform-core) |

Invariants: request validation before any socket; trait default methods
return `Err(Unsupported(capability))` for capabilities absent from
`ProviderInfo`; platform performs no retry/fallback (caller-owned, mirrors
alltools keeping `fallback/` product-side); `Ok` means the provider HTTP
call completed with a usable response body, nothing about semantics.

## 5. Contract and compatibility

- Public surface (re-exported): all §4 types, `AiError`, `AiProvider`,
  `EmbeddingProvider`, `mock::MockProvider` (feature `mock`),
  `openai_compat::{OpenAiCompatConfig, OpenAiCompatProvider}` (feature
  `openai-compat`), `ollama::{OllamaConfig, OllamaProvider}` (feature
  `ollama`).
- `OpenAiCompatConfig`: `api_key: Secret<String>` (required), `base_url:
  bounded String` default `https://api.openai.com`, `chat_path` default
  `/v1/chat/completions`, `embed_path` default `/v1/embeddings` (path
  override covers zhipu/deepseek drift), `timeout: Duration` (default 60 s),
  `max_stream_bytes` (default 8 MiB). HTTPS enforced for non-loopback hosts
  at construction (`Err(Configuration)` otherwise).
- `OllamaConfig`: `base_url` default `http://127.0.0.1:11434`, `timeout`
  default 120 s (local models are slow), paths `/api/chat` + `/api/embed`.
- Wire mapping: chat uses `messages`/`tools`/`tool_calls` (OpenAI
  06-2025-shape pinned at implementation as a test fixture); streaming sets
  `stream: true` and parses `text/event-stream` lines, ignoring
  `[DONE]`; structured output requests send
  `response_format: {"type":"json_schema", "json_schema": {"strict":true,…}}`;
  ollama uses `/api/chat` NDJSON with `"format": <schema>`.
- `AiError` → existing `ErrorCategory` (enum frozen, mailing L3 inherited):
  `Validation(String)` → Invalid; `Configuration(String)` → Invalid;
  `ContentPolicy(reason)` → Invalid; `Unsupported(Capability)` → Invalid;
  `Unauthorized` → Unavailable; `RateLimited(retry_after_secs: Option<u64>)`
  → Unavailable; `TransportUnavailable(reason)` → Unavailable; `Timeout` →
  Unavailable; `ProviderServer(status, safe_detail)` → Unavailable;
  `MalformedResponse(reason)` → Unavailable. `Display` omits keys, prompts,
  completions, and raw bodies (truncated server `error.message` only, ≤ 200
  bytes, control chars stripped).
- Serde derives on public types (consumers persist session transcripts);
  `Secret` types have no `Serialize` impl (compile-time redaction).

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| 0 messages / >256 / temperature out of range | `Err(Validation)`, no socket |
| HTTP 401/403 | `Err(Unauthorized)`; key redacted |
| HTTP 429 | `Err(RateLimited(retry_after))` parsed from header, no auto-retry |
| HTTP 4xx content refusal | `Err(ContentPolicy)`; `FinishReason::ContentRefused` when returned in-body |
| HTTP 5xx / connection refused / DNS | `Err(ProviderServer/TransportUnavailable)`; no retry |
| Request/response exceeds `timeout` | `Err(Timeout)` (tokio time via reqwest only) |
| Malformed JSON / missing choices | `Err(MalformedResponse)` |
| Stream ends before final `Turn` | `Err(MalformedResponse("stream-truncated"))` |
| Capability not declared (e.g. embed on ollama-only mock) | default trait method `Err(Unsupported)` |
| Plain http to non-loopback remote (openai-compat) | `Err(Configuration)` at construction |
| Default features | crate compiles and tests pass; no reqwest/tokio/futures edges |

No partial success: one request, one result (or stream-terminating error).

## 7. Verification oracle

- Type/contract units (`tests/ai_types.rs`): builder validation boundary
  rows from §6 (257 messages, 2.1 temperature, 0 stop sequences ok/over 4,
  empty embed batch); secret non-serialization test.
- Mock tests (`tests/ai_mock.rs`): scripted success/rate-limit/timeout;
  unsupported-capability path via declared `Capabilities::none()`.
- Adapter HTTP tests (`tests/ai_openai_compat.rs`, `tests/ai_ollama.rs`):
  `wiremock` in-process servers replaying canned fixtures — happy chat,
  tools round-trip, SSE stream with `[DONE]`, structured json_schema body,
  embeddings batch, plus negatives 401/429(+Retry-After)/500/malformed/
  stream-truncation; assertions check the mapped `AiError` variant and that
  `Display` output contains no fixture secret string (`assert_not_contains`
  helper local to the test file).
- Error-mapping completeness: pure `fn status_to_error(status, …)` tested
  over a table covering 401/403/404/422/429/5xx/other.
- Feature matrix: §2 commands green; `cargo tree -p platform-ai
  --no-default-features` shows no reqwest/tokio/futures; `--features
  openai-compat` adds reqwest; `--all-features` builds.
- Architecture: allowlist table extended; existing rule function plus a new
  unit case — synthesized graph attaching `reqwest` to `platform-identity`
  must fail; `platform-ai` reaching reqwest must pass.
- Evidence before any task box ticks: command + result recorded in HANDOFF;
  `openspec validate --changes --strict --no-interactive` green.

## 8. Decision ledger

| # | Decision | Choice | Alternatives rejected |
|---|---|---|---|
| L1 | Crate home | satellite `platform-ai` | five vendor crates (fragmentation); per-consumer duplication (status quo) |
| L2 | Vendor unification | ONE `openai-compat` adapter + path overrides | per-vendor adapters (donor evidence: deepseek/zhipu differ only in base_url/model) |
| L3 | Capability model | `ProviderInfo` + declared bits with default `Unsupported` methods | one trait per capability (breaks dyn-singleton injection) |
| L4 | Token counts | provider-reported `Usage` only | tiktoken integration (heavy local dep, alltools keeps it product-side) |
| L5 | Anthropic | deferred, zero implemented consumers | shipping from otk declarations alone |
| L6 | Budget growth | table row only; rule function untouched | rewriting the oracle |
| L7 | Error style | hand-rolled Display on platform-core | thiserror (breaks mailing precedent mid-batch) |
| L8 | Streaming chunk shape | `Text` deltas + exactly-one final `Turn` | donor `LlmResponse`-per-chunk (ambiguous finality); b) multimodal content parts deferred (no consumer sends images today) |
| — | Blockers | none; D4 publication is the separate `registry-publication` change | — |

## Traceability

| Req | Proposal outcome | Design section | Boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| AI-001 chat contract + validation + redaction | What Changes #1 | §4 §5 §6 | types.rs, error.rs, provider.rs | success/failure/boundary | 2.1–2.3 | §7 types+mapping tests |
| AI-002 capability declaration + unsupported defaults | What Changes #1 | §4 §5 | provider.rs | success/failure | 2.3 | §7 mock unsupported path |
| AI-003 openai-compat adapter | What Changes #2 | §2 §5 §6 | openai_compat.rs | success/failure/boundary | 2.4 | §7 wiremock fixtures |
| AI-004 ollama adapter | What Changes #2 | §2 §5 §6 | ollama.rs | success/failure/boundary | 2.5 | §7 wiremock fixtures |
| AI-005 deterministic mock | What Changes #1 | §4 §6 | mock.rs | success/boundary | 2.6 | §7 mock tests |
| AI-006 satellite budget quarantine | What Changes #3 | §3 §7 | platform-arch-tests | success/failure/boundary | 2.7 | §7 arch unit case |
