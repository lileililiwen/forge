# Proposal: A shared AI provider capability crate

## Why

Two consumer repositories already implement provider-client seams for LLM
calls. `alltools-platform` carries five independent provider
implementations (`rust/packages/at-domain-agent/src/providers/` — openai,
deepseek, zhipu, ollama, local) whose remote three differ only by
`base_url` and model id (evidence: `openai.rs:217` builds
`{base_url}/v1/chat/completions` for every OpenAI-compatible vendor), plus a
provider-neutral lifecycle port in `at-domain-ai/src/domain/invocation.rs`;
`openTeacherKit` owns `crates/otk-ai/src/provider.rs` with a
`ProviderDeclaration` capability model (tasks, retention, failure kinds)
currently backed only by scripted adapters. `platform-mailing` established
the satellite-crate and allowlist pattern this change extends. The recorded
two-consumer decision for the AI seam is made here, per the repository
deferral policy.

## What Changes

- New opt-in crate `platform-ai`: provider-neutral chat/structured/embedding
  types, one `AiProvider` trait with capability declaration (streaming,
  tools, structured output) plus a separate `EmbeddingProvider` trait, typed
  errors mapped onto existing `platform-core` `ErrorCategory`, and an
  in-crate `mock` feature for consumer tests.
- Adapters behind cargo features: `openai-compat` (covers OpenAI, DeepSeek,
  Zhipu and any configurable-`base_url` vendor, including streaming,
  tool calls, JSON-schema structured output, and embeddings) and `ollama`
  (native `/api/chat` and `/api/embed` for local models).
- `platform-arch-tests` allowlist table extended with the named satellite
  `platform-ai` and its bounded dependency budget (`reqwest`, `tokio`,
  `futures`); foundation neutrality is unchanged.
- Workspace additions: `reqwest`/`futures` workspace dependencies, crate
  member registration, README/ROADMAP/HANDOFF status at implementation time.

## Package Boundary and Split Assessment

**Single outcome:** a consumer can call an LLM provider through one typed,
provider-neutral Rust contract without duplicating HTTP plumbing.

**Included:** contract types, capability declaration, error taxonomy, mock,
OpenAI-compatible adapter, ollama adapter, allowlist extension.

**Excluded (split signals):** Anthropic adapters have zero implemented
consumers (openTeacherKit only declares it) — deferred; circuit
breaker/retry/traceparent resilience is a distinct lifecycle owned by the
future `transport-resilience` change (`platform-http`); token counting for
cost is provider-reported `Usage` only, tiktoken-style local counting stays
with consumers; agent/ReAct loops remain in alltools (one consumer);
consumer migrations are each consumer's own change.

**Dependencies:** B1 shipped the satellite precedent and the allowlist
mechanism. Publication to crates.io (roadmap D4) is the separate
`registry-publication` change and does not block this crate existing.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| alltools-platform | `at-domain-agent/src/providers/{traits,openai,ollama,factory}.rs` | `AIProvider` shape (generate/stream/structured/embed + tool calls), base_url-driven vendor unification | product governance errors (Budget/Quota/Hallucination) inside `ProviderError`; tracing/logging inside provider code | alltools owns product; rust-platform-libs owns contract | **extract recurring capability** (client contract + adapters only) |
| alltools-platform | `at-domain-ai/src/domain/invocation.rs` | provider-neutral invocation/embedding port vocabulary | typestate lifecycle is product-specific | alltools | **adapt through a generic adapter** (vocabulary only) |
| openTeacherKit | `crates/otk-ai/src/provider.rs` (`ProviderDeclaration`: tasks, retention, failure kinds) | capability-declaration concept for the trait's `ProviderInfo` | sync trait, no HTTP; safety-boundary fields are product policy | openTeacherKit | **adopt pattern**; otk migrates in its own change |
| openpanel | `crates/openpanel-app/src/identity/sso/` etc. | none for AI clients | — | openpanel | **not applicable** |
| this repository | `crates/platform-mailing/` (archived `2026-09-30`), `platform-arch-tests` allowlist | satellite crate layout, feature-gating, in-crate mock, error-to-`ErrorCategory` precedent | budget must grow for reqwest/futures | rust-platform-libs | **extend shared owner** |
| platform-contracts | `capability` schema family | registry vocabulary for a future `ai-providers` capability entry | registry is roadmap D3/B4 work | platform-contracts | **not applicable** (reference only) |

## BFS Impact Map

- **Capabilities:** new `platform-ai`; arch-test allowlist table; README
  implemented/deferred lists; ROADMAP deferral note for the AI seam.
- **Unaffected:** foundation four crate APIs, `platform-mailing`,
  `platform-testing`, every sibling repository, CI semantics.
- **Callers:** none in-tree; consumers adopt later under their own changes
  (alltools `ai-platform-conformance`, openTeacherKit
  `provider-contract-conformance` — roadmap B1.5 exit evidence).
- **Failure/boundary:** 401/429/5xx/timeout/malformed-body/refused-content —
  each maps to one typed `AiError`; no implicit retry or fallback in the
  platform (alltools `fallback/` stays product-owned).
- **Compatibility:** default features off ⇒ crate compiles without reqwest;
  existing `ErrorCategory` enum untouched; wire shapes follow the published
  OpenAI chat-completions/embeddings revisions pinned at implementation.
- **Security/privacy:** API keys are `Secret<…>` (never `Display`); prompts,
  completions, and server bodies MUST NOT appear in error text or tracing;
  adapters add no telemetry; ollama adapter defaults to loopback hosts.

## Capabilities

- `platform-ai`

## Non-goals

- No Anthropic/local-quant adapters, no agent loop, no token counting, no
  retry/circuit-breaker, no cost governance, no streaming past first token
  backpressure design beyond the declared chunk contract.
- No consumer repository edits and no adoption claims.
- No publication policy change (`publish = false` stands until
  `registry-publication`).
- No `ErrorCategory` enum extension.

## Evidence boundary

This change proves the contract compiles across its feature matrix, adapters
parse canned OpenAI-compatible and ollama responses correctly (wiremock
fixtures, no live providers), error mapping is total, mocks are
deterministic, and the dependency-direction oracle still quarantines the
foundation. It cannot prove vendor wire compatibility against production
endpoints or consumer migration; that arrives with the per-consumer changes
and their own gate runs.
