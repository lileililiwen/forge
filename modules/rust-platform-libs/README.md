# Rust Platform Libraries

Small, opt-in Rust crates for shared cross-cutting contracts used by multiple
web, operator, and control-plane products. This is not an application
framework and does not own product databases, users, migrations, or UI.

The foundation, the second-layer identity/audit/permission surface, the
operational payload surface, the opt-in SMTP mailing satellite, and the
opt-in AI capability satellite are implemented and spec-synced. Axum,
SQLx, cookie, OIDC, HTTP mail provider adapters, Anthropic, and
persistence remain deferred until real consumers demonstrate compatible
contracts.
The registry-publication pipeline is implemented; the first
`v0.1.0` release remains BLOCKED on repository availability, owner approval
and the registry token described in `docs/publication.md` §Pre-publish checklist.
See `HANDOFF.md` for the current state and the next OpenSpec change.

## Crate matrix

Every crate inherits `version`, `license`, `publish`, `rust-version`,
and `edition` from the workspace `Cargo.toml`; the matrix restates
them so a consumer never has to open a manifest.

| Crate | Purpose | Posture | Version | License | Publish | MSRV | Edition |
|---|---|---|---|---|---|---|---|
| `platform-core` | Bounded strings, opaque IDs, clocks, correlation IDs, redaction, safe errors | production | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-contracts` | Rust representations of the language-neutral schemas and the JSON-Schema-driven compatibility validator | production | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-identity` | Provider-neutral subjects, sessions, permissions, tenants, lifecycle, and the evaluator/resolver trait surface | production | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-audit` | Normalized audit events, redaction policy, sink trait, and event producers | production | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-testing` | Deterministic fakes, builders, and assertions for consumer test suites | test-only | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-arch-tests` | Workspace-level dependency-direction tests | test-only | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-mailing` | Provider-neutral mail send contract, opt-in lettre SMTP adapter (`smtp-lettre`), deterministic mock (`mock`) | satellite (production, opt-in; default features empty) | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |
| `platform-ai` | Provider-neutral AI capability contract (`AiProvider`, `EmbeddingProvider`, `ChatRequest`, `StreamChunk`, `AiError`); opt-in OpenAI-compatible HTTP adapter (`openai-compat`), native ollama adapter (`ollama`), and deterministic in-crate mock (`mock`) | satellite (production, opt-in; default features empty) | `0.1.0` | `Apache-2.0` (workspace) | `false` | `1.75` | `2021` |

All crates are version `0.1.0`. The seven publishable crates
(`platform-core`, `platform-contracts`, `platform-identity`,
`platform-audit`, `platform-testing`, `platform-mailing`,
`platform-ai`) are ready for registry publication; the workflow,
metadata, and per-crate exclusion list are in place, and the first
`v0.1.0` release remains BLOCKED on the preconditions in
`docs/publication.md` §Pre-publish checklist. The internal
`platform-arch-tests` crate stays `publish = false`: it is
workspace-internal tooling, not a consumer-facing library.
Until publication lands, depending on a crate means a path or git
dependency (see Adoption below), never a version from crates.io.

## Usage

`docs/usage.md` gives one minimal, compile-checked example per
production crate, each backed by a file under `crates/*/examples/`
that `cargo test --workspace` builds. Test-only crates are excluded.

## License

The workspace declares `license = "Apache-2.0"` and every crate
inherits it (`license.workspace = true`). The license text lives at
the repository root in `LICENSE`.

## Adoption

The only proven adoption route is a path dependency, as used by the
in-workspace `tests/consumer-fixture` crate:

```toml
[dependencies]
platform-core = { path = "../../crates/platform-core" }
```

The shared lint, MSRV, CI, and conformance recipe for adopting the
platform is owned by the `rust-platform-consumer-baseline` change
(spec `openspec/specs/rust-platform-consumer-baseline/spec.md`,
recipe `docs/consumer-baseline.md`). This README does not re-declare
that recipe; follow it verbatim instead of forking it.

## Implemented crates

- `platform-core` — bounded results, errors, IDs, clocks, redaction, and
  correlation contracts.
- `platform-contracts` — Rust representations of the language-neutral
  schemas and the JSON-Schema-driven compatibility validator.
- `platform-identity` — provider-neutral `Subject`, `Session`, `Permission`,
  `Tenant`, `Lifecycle`, `IdentityError`, `IdentityOutcome`, and the
  `IdentityStore` / `IdentityProvider` / `CurrentSubject` traits. The
  second-layer surface adds `PermissionEvaluator` (with
  `DenyByDefaultEvaluator` and `AllowListEvaluator` reference impls) and
  `TenantResolver` (with `NoTenantResolver` placeholder). No persistence,
  no provider config, no HTTP.
- `platform-audit` — normalized `AuditEvent`, `AuditEventBuilder`,
  `RedactedContext` (rejects secret-named keys at construction), the
  `AuditSink` trait with `NoopAuditSink` and `InMemoryAuditSink`, and the
  `AuthenticationAuditor`, `PermissionAuditor`, and `SessionAuditor`
  event producers. No storage, no transport, no retention policy.
- `platform-testing` — deterministic `FakeClock`, the `TestSubject`
  builder, `assert_redacted!` helper, and the
  `ScriptedPermissionEvaluator` test double. Production crates MUST NOT
  depend on this crate; `platform-arch-tests` enforces it.
- `platform-arch-tests` — workspace-level dependency-direction tests that
  read the `Cargo.lock` and assert no foundation production crate (directly or
  transitively) depends on `axum`, `sqlx`, `cookie`, `jsonwebtoken`,
  `openidconnect`, `oauth2`, `reqwest`, `lettre`, `tokio`, or `platform-testing`.
  The named satellites `platform-mailing` and `platform-ai` are each
  explicitly allowlisted: `platform-mailing` may reach `lettre`/`tokio`;
  `platform-ai` may reach `reqwest`/`tokio`/`futures`. Any other crate
  reaching them fails until a policy change names it.
- `platform-mailing` — provider-neutral `EmailAddress`, `EmailMessage`,
  `Body`, `AttachmentRef`, `MailTransport`, `SendContext`,
  `DeliveryReceipt`, and `MailingError` (every variant mapped onto a shipped
  `platform-core` `ErrorCategory`, redacted display). The `smtp-lettre`
  feature adds `SmtpTransportConfig` (loopback-only `Security::None`,
  `Secret`-wrapped credentials) and `LettreSmtpTransport` (rustls only, no
  native-tls); the `mock` feature adds the scriptable `MockTransport` so
  production crates never touch `platform-testing`. No retry, queue,
  persistence, templates, or HTTP provider adapters.
- `platform-ai` — provider-neutral `ChatRequest`, `ChatResponse`,
  `StreamChunk`, `Embedding`, `Message`, `ToolDefinition`, `ToolCall`,
  `GenerationOptions`, `Capabilities`, `ProviderInfo`, `AiContext`, and
  `AiError` (every variant mapped onto a shipped `platform-core`
  `ErrorCategory`, redacted display; no key/prompt/completion leaks in
  `Display`). The `openai-compat` feature adds `OpenAiCompatConfig` and
  `OpenAiCompatProvider` (rustls only, no native-tls; SSE streaming;
  JSON-schema structured output; loopback-or-https `base_url` only); the
  `ollama` feature adds `OllamaConfig` and `OllamaProvider` (NDJSON
  streaming, default loopback endpoint, no credentials); the `mock`
  feature adds `MockAi` so production crates can script outcomes
  without touching `platform-testing`. No Anthropic, no retry, no
  agent loop, no local token estimation.

## Deferred

Axum, SQLx, cookie, OIDC, universal login pages, dashboards, product
entities, migrations, provider credentials, billing semantics, and
deployment automation remain owned by consumers. HTTP mail provider
adapters (Resend/Brevo: one consumer — the two-consumer rule says wait),
router/quota/rate-limit, templates, DKIM/IMAP, persistence adapters,
and the Anthropic provider adapter (zero implemented consumers)
land after compatible-contract evidence exists and an explicit OpenSpec
change records the decision. SMTP delivery and the
OpenAI-compatible + ollama AI surface are the two shipped satellites;
the foundation crates stay provider-neutral behind the trait surface.

## Verification

The verification command is `cargo test --workspace`. The full
verification surface (recorded in `HANDOFF.md`) also includes
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `openspec validate --specs --strict --no-interactive`,
and the in-tree 108-case cross-repository fixture parity walker in
`crates/platform-contracts/tests/fixture_parity.rs` (9 of 9 roadmap
contract families × 12 canonical cases).

Build/test/skeleton success is not runtime or release evidence. Each
completed OpenSpec spec/change requires exactly two commits: first the
implementation/tests/archive commit, then a `HANDOFF.md`-only
pointer/evidence commit.

Use OpenSpec for every non-trivial change and follow BFS → DFS → BFS.
