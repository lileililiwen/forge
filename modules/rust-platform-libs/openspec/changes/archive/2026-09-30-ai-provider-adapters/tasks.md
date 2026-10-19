# Tasks: platform-ai crate and satellite budget extension

## 0. Caveats and confirmations

- [ ] **Do not edit sibling repositories.** alltools-platform and
  openTeacherKit migrate in their own OpenSpec changes.
- [ ] **Foundation neutrality is restated, not weakened.** reqwest/tokio/
  futures may be reachable ONLY from `platform-ai`.
- [ ] **`ErrorCategory` enum stays frozen** (mailing L3 inherited). A mapping
  conflict against shipped variants is a STOP condition, not an enum edit.
- [ ] **No retry/fallback/agent-loop content** — those are product-owned
  (alltools `fallback/`, roadmap B2 deferrals).
- [ ] **Keep `publish = false`.** Crates.io publication is the separate
  `registry-publication` change (roadmap D4).
- [ ] **No live provider calls in tests.** wiremock fixtures only; CI has no
  API keys and must not need them.
- [ ] **Record environment-blocked checks** as BLOCKED with the exact
  command; never as passing.

## 1. BFS — Baseline and impact coverage

- [ ] Read `openspec/specs/platform-mailing/spec.md`, the satellite layout of
  `crates/platform-mailing/` (features, in-crate mock, error style), and the
  `platform-arch-tests` allowlist table as shipped.
- [ ] Map every §6 failure row of design.md to its test file/case from §7;
  confirm proposal ↔ design ↔ spec consistency.
- [ ] Confirm `reqwest` (rustls-tls, stream, http2), `futures`, and
  `wiremock` resolve as workspace deps without introducing native-tls or
  foundation-visible edges (`cargo add --dry-run` equivalent).
- [ ] Verify the implementation-handoff gate: satellite home, toolchain,
  allowlist mechanism, and frozen-error policy are as in design §1–§3.
- [ ] Add test skeletons for §4 validation and `status_to_error` (compile-
  level only; no behavior claimed).

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 AI-001: `types.rs` — `Role`, `Message`, `Content`, `ChatRequest`,
  `GenerationOptions`, `ToolDefinition`, `ToolCall`, `FinishReason`,
  `Usage`, `Embedding`, `StreamChunk` with §4 bounds and §6 validation.
- [ ] 2.2 AI-001: `error.rs` — `AiError` variants, total §5 mapping onto
  existing `ErrorCategory`, redaction tests (keys/prompts/bodies absent from
  `Display`).
- [ ] 2.3 AI-001/002: `provider.rs` — `ProviderInfo`, `Capabilities`,
  `AiProvider` (async-trait: `info`, `generate`, default-`Unsupported`
  `stream`/`generate_structured`), `EmbeddingProvider`, `AiContext`.
- [ ] 2.4 AI-003: `openai_compat.rs` under feature `openai-compat` — config
  (Secret key, base_url/path overrides, loopback-only-http rule, timeout,
  stream byte cap), chat/tools/structured request builders, SSE parsing to
  `StreamChunk` with mandatory final `Turn`, embeddings batch, §6 status
  mapping; wiremock tests per design §7.
- [ ] 2.5 AI-004: `ollama.rs` under feature `ollama` — `/api/chat` NDJSON
  streaming, `"format"` schema structured mode, `/api/embed` embeddings,
  no-auth config; wiremock tests per design §7.
- [ ] 2.6 AI-005: `mock.rs` under feature `mock` — scripted
  success/rate-limit/timeout/truncation outcomes, declarable
  `Capabilities`, clock-injected timestamps, zero network.
- [ ] 2.7 AI-006: `platform-arch-tests` — add the `platform-ai` allowlist
  row; new unit case: synthesized graph with reqwest on `platform-identity`
  fails, `platform-ai` passes; register crate + workspace deps in root
  `Cargo.toml`.

## 3. BFS — Cross-surface regression and completeness

- [ ] Run the full §2 feature matrix; every §6 row has a passing test or a
  recorded BLOCKED reason.
- [ ] `cargo tree -p platform-ai --no-default-features` proves absence of
  reqwest/tokio/futures; all-features build proves presence.
- [ ] Re-inspect the four foundation crates and `platform-mailing` for zero
  API drift; update README (platform-ai to Implemented, deferred list
  trimmed), ROADMAP (record the two-consumer extraction decision), and
  confirm no placeholder/TBD text remains.
- [ ] Confirm no sibling repository was modified.

## 4. Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`
- [ ] `cargo test -p platform-ai --no-default-features` and
  `cargo test -p platform-ai --features openai-compat,ollama,mock`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `node scripts/check-openspec-change-names.mjs`
- [ ] `openspec validate --changes --strict --no-interactive`
- [ ] Record evidence in `HANDOFF.md`; completion follows the repo's
  two-commit cadence (implementation+archive, then HANDOFF pointer).
