# Rust Platform Libraries Handoff

current_spec: none

This file preserves the source project's delivery history as of the imported
revision. Forge's root `HANDOFF.md` owns active work, module import evidence,
and the current OpenSpec pointer. The source release pipeline is present in
this module, but no owner approval, tag, credential, or crates.io release is
claimed by the import.

## Current state

Seven OpenSpec changes are **implemented, archived, and
spec-synced** in this repository.

1. `rust-platform-foundation-and-contracts` (archived on 2026-09-24
   to `openspec/changes/archive/2026-09-24-rust-platform-foundation-and-contracts/`,
   spec at `openspec/specs/rust-platform-foundation/spec.md`).
2. `identity-audit-and-permission-contracts` (archived on
   2026-09-24 to
   `openspec/changes/archive/2026-09-24-identity-audit-and-permission-contracts/`,
   spec at `openspec/specs/rust-identity-audit-permission/spec.md`).
3. `rust-platform-consumer-baseline` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-rust-platform-consumer-baseline/`,
   spec at `openspec/specs/rust-platform-consumer-baseline/spec.md`).
4. `consumer-conformance-matrix` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-consumer-conformance-matrix/`,
   spec synced into `openspec/specs/rust-platform-foundation/spec.md`).
5. `post-mvp-readiness` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-post-mvp-readiness/`,
   spec at `openspec/specs/readiness/spec.md`).
6. `mailing-provider-adapters` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-mailing-provider-adapters/`,
   spec at `openspec/specs/platform-mailing/spec.md`).
7. `ai-provider-adapters` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-ai-provider-adapters/`,
   spec at `openspec/specs/platform-ai/spec.md`).
8. `registry-publication` (archived on 2026-09-30 to
   `openspec/changes/archive/2026-09-30-registry-publication/`,
   spec at `openspec/specs/rust-platform-publication/spec.md`).
   The pipeline is implemented; the first `v0.1.0` release is
   BLOCKED on P6 (real GitHub-publication precondition + owner
   approval).

What is in the tree:

* `Cargo.toml` — workspace with seven crates:
  * `platform-core` — bounded strings, opaque `Id<T>`, `Clock` trait
    plus `SystemClock` / `FixedClock`, `CorrelationId`, `Secret<T>`
    redaction wrapper, and the `ErrorCategory` / `SafeError` model.
  * `platform-contracts` — `Envelope`, `ContractRegistry` loader,
    JSON-Schema-driven compatibility validator (Draft 2020-12 via the
    `jsonschema` crate), and Rust payload types for the nine
    language-neutral contract families.
  * `platform-identity` — provider-neutral `Subject`, `Session`,
    `Permission`, `Tenant`, `Lifecycle`, `IdentityError`,
    `IdentityOutcome`, `IdentityStore` / `IdentityProvider` /
    `CurrentSubject` traits, plus the second-layer `PermissionEvaluator`
    (with `DenyByDefaultEvaluator` and `AllowListEvaluator` reference
    impls) and `TenantResolver` (with `NoTenantResolver` placeholder).
    No persistence, no provider config, no HTTP.
  * `platform-audit` — `AuditEvent`, `AuditEventBuilder`,
    `RedactedContext` (rejects secret-named keys at construction), the
    `AuditSink` trait with `NoopAuditSink` and `InMemoryAuditSink`, and
    the `AuthenticationAuditor`, `PermissionAuditor`, and
    `SessionAuditor` event producers.
  * `platform-testing` — test-only `FakeClock`, `TestSubject`
    builder, `assert_redacted!` helper, and the
    `ScriptedPermissionEvaluator` test double. Production crates
    MUST NOT depend on this crate; `platform-arch-tests` enforces
    it.
  * `platform-arch-tests` — workspace-level dependency-direction
    tests that read the `Cargo.lock` and assert no foundation
    production crate (directly or transitively) depends on `axum`,
    `sqlx`, `cookie`, `jsonwebtoken`, `openidconnect`, `oauth2`,
    `reqwest`, `lettre`, `tokio`, or `platform-testing`, while the
    named satellites `platform-mailing` and `platform-ai` are
    each explicitly allowlisted: `platform-mailing` may reach
    `lettre`/`tokio`; `platform-ai` may reach `reqwest`/`tokio`/
    `futures`. Any other crate reaching those leaves is denied
    until an explicit policy change names it.
  * `platform-mailing` — satellite (production, opt-in) mail send
    contract: `EmailAddress`, `EmailMessage`, `Body`,
    `AttachmentRef`, `MailTransport`, `SendContext`,
    `DeliveryReceipt`, `MailingError` (frozen-category mapping,
    redacted display), the `smtp-lettre` adapter
    (`SmtpTransportConfig`, `LettreSmtpTransport`, rustls only),
    and the `mock` feature's `MockTransport`. No retry, queue,
    persistence, templates, or HTTP provider adapters.
  * `platform-ai` — satellite (production, opt-in) AI capability
    contract: `Message`, `ChatRequest`, `GenerationOptions`,
    `ToolDefinition`, `ToolCall`, `ChatResponse`, `StreamChunk`,
    `Embedding`, `Capabilities`, `ProviderInfo`, `AiContext`,
    `AiProvider` / `EmbeddingProvider`, `AiError` (frozen-category
    mapping, redacted display), the `openai-compat` adapter
    (`OpenAiCompatConfig`, `OpenAiCompatProvider`, rustls only,
    SSE streaming, JSON-schema structured output, tool calls;
    covers OpenAI, DeepSeek, Zhipu, and any other vendor that
    exposes the same wire shape), the `ollama` adapter
    (`OllamaConfig`, `OllamaProvider`, default loopback endpoint,
    NDJSON streaming, no credentials), and the `mock` feature's
    `MockAi`. No Anthropic, no retry, no agent loop, no local
    token estimation.
* `crates/platform-contracts/tests/fixture_parity.rs` — the
  cross-repository fixture walker. The test resolves the live
  `platform-contracts` repository via `PLATFORM_CONTRACTS_DIR` (or
  `../platform-contracts` relative to the workspace root) and walks
  the canonical fixture matrix.
* `crates/platform-testing/tests/identity_permission_audit_e2e.rs` —
  the end-to-end identity → permission → audit smoke test that
  asserts the new producer surface and the new evaluator trait
  surface compose into the same canonical audit event that the
  `platform-contracts` schema expects.

## Evidence (recorded 2026-09-24)

* `node scripts/check-openspec-change-names.mjs` → ok.
* `cargo fmt --all -- --check` → clean.
* `cargo test --workspace` → **90 passed, 0 failed** (69 foundation
  tests + 21 new tests across the permission evaluator, tenant
  resolver, audit producers, scripted test double, and the
  end-to-end identity → permission → audit smoke test).
* `cargo clippy --workspace --all-targets -- -D warnings` → clean.
* `openspec validate --changes --strict --no-interactive` → 1/1
  passed (`identity-audit-and-permission-contracts`).
* `openspec validate --specs --strict --no-interactive` → 2/2 passed
  (`spec/rust-platform-foundation` 3 requirements / 5 scenarios;
  `spec/rust-identity-audit-permission` 4 requirements / 4
  scenarios).
* Cross-repository fixture parity: **48 cases × 4 contract families
  pass** (`identity-subject`, `permission-decision`,
  `tenant-context`, `audit-event`). Every valid, invalid, boundary,
  unknown-field, redaction, and compatibility outcome in the
  language-neutral `platform-contracts` matrix is matched by the
  Rust validator.

The complete implementation commit is `33917c2` on `main`
("Implement identity-audit-and-permission-contracts"). The
foundation implementation commit is `30818c5`; the foundation
HANDOFF pointer is `73a8f88`. The change archives are
`openspec/changes/archive/2026-09-24-rust-platform-foundation-and-contracts/`
and
`openspec/changes/archive/2026-09-24-identity-audit-and-permission-contracts/`,
and the synced specs are `openspec/specs/rust-platform-foundation/spec.md`
and `openspec/specs/rust-identity-audit-permission/spec.md`.

## Evidence — rust-platform-consumer-baseline (recorded 2026-09-30)

The change is implemented and verified; the archive step is commit 1
of the two-commit cadence below.

What was added (no crate public API, spec, or consumer change):

* `rustfmt.toml` — stable-only settings; `cargo fmt --check` is
  warning-free on the pinned toolchain (nightly-only
  `imports_granularity` / `group_imports` were tried and removed for
  exactly this reason).
* `clippy.toml` — `msrv = "1.75"` matching the declared MSRV.
* `rust-toolchain.toml` — pin `channel = "1.95.0"`. The first choice,
  `1.85.0`, could not build the workspace (`time 0.3.55` requires
  rustc >= 1.88); the declared MSRV stays `1.75` per maintainer
  decision and the effective 1.88 floor is recorded in
  `docs/consumer-baseline.md` §2. Raising the declared MSRV is a
  separate change with a migration note.
* `deny.toml` — `cargo deny` baseline (permissive-license allowlist,
  RustSec advisories, wildcard versions denied with
  `allow-wildcard-paths = true` so intra-workspace path deps pass,
  crates.io only). Passes with one standing warning (duplicate
  `getrandom` 0.3.4/0.4.3 via `jsonschema` vs `uuid`); duplicates are
  `warn`, not `deny`.
* `.github/workflows/ci.yml` — extended from tests-only to
  formatting, clippy `-D warnings`, `cargo deny check`, tests, and
  `./scripts/conformance.sh`, all on the pinned toolchain (no
  toolchain input; `rust-toolchain.toml` governs).
* `docs/consumer-baseline.md` — the adoption recipe (in-workspace
  path form, out-of-workspace versioned git form, MSRV policy, CI
  table, harness proofs, non-goals).
* `scripts/conformance.sh` — six proofs, all holding: pinned
  toolchain identity, in-workspace fixture green with
  workspace-relative paths only, out-of-workspace scratch crate
  resolving/building/testing/formatting/linting clean under the
  verbatim-copied baseline, and three negative fixtures failing
  loudly with named causes (`unused-variables`, `requires rustc
  1.99`, banned `serde`).
* `tests/consumer-fixture` — workspace-member consumer proving the
  path recipe end to end (`TestSubject` → `DenyByDefaultEvaluator` →
  `PermissionAuditor` → `InMemoryAuditSink`, asserting
  `Permission`/`Denied`).

Verification outputs (pinned toolchain 1.95.0, all fresh this session):

* `cargo test --workspace` → **91 passed, 0 failed** (90 prior + 1
  fixture).
* `cargo fmt --all -- --check` → clean.
* `cargo clippy --workspace --all-targets -- -D warnings` → clean.
* `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok.
* `./scripts/conformance.sh` → all proofs hold.
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → 3/3 passed.
* `openspec validate --specs --strict --no-interactive` → 2/2 passed.
* `platform-arch-tests` passes unchanged (run inside `cargo test
  --workspace`); `[workspace.lints]` untouched; `publish = false`
  unchanged; no sibling repository touched.
* Commit note: `.github/workflows/ci.yml` existed only as untracked
  tests-only scaffolding; this change extends it to the enforced
  baseline above and commits it. The pre-existing `.project.json`
  modification stays out of this change's commits per the task
  caveats.

Evidence boundary for this change: the harness proves the recipe
resolves, builds, tests, formats, lints, and fails loudly with named
causes. It does not prove any consumer has adopted the baseline in
production, and the out-of-workspace probe uses an absolute path
dependency as an explicitly documented offline stand-in for the
versioned git form (no registry or git remote exists in this
environment).

## Evidence — consumer-conformance-matrix (recorded 2026-09-30)

The change is implemented, archived to
`openspec/changes/archive/2026-09-30-consumer-conformance-matrix/`,
and spec-synced into `openspec/specs/rust-platform-foundation/spec.md`
(new requirement: operational families have typed payloads and parity
walkers). Implementation commit is `9128c35`.

What was added (no validator, registry, envelope, schema, or other-crate
change; no new dependencies):

* `crates/platform-contracts/src/payload/{gate_result,job_outcome,release_evidence,capability}.rs`
  — `GateResult`/`GateCheck`/`GateResultValue`,
  `JobOutcome`/`JobResult`, `ReleaseEvidence`/`EvidenceKind`,
  `Capability`/`Stability`, following the existing permissive-builder
  payload pattern (serde structs, snake_case enums, `Option` for
  schema-optional fields, raw-`String` timestamps), each with a
  serde round-trip unit test and a `ContractPayload` impl.
* `crates/platform-contracts/tests/fixture_parity.rs` — five walker
  tests reusing the unchanged `walk_family` helper; all 5 families
  pass on the unmodified generic validator with zero policy changes.
* `crates/platform-contracts/tests/typed_payload_parity.rs` — four
  tests proving each new typed payload serializes to an envelope the
  registry-backed validator accepts.
* `README.md` parity count (48 → 108 cases), `ROADMAP.md` step 4
  marked done.

Verification outputs (all fresh this session):

* `cargo test --workspace` → **104 passed, 0 failed** (91 prior + 5
  walkers + 4 round-trips + 4 typed-parity).
* `cargo test -p platform-contracts` → 13 unit + 11 parity + 4
  typed-parity, all green.
* Cross-repository fixture parity: **108 cases × 9 roadmap contract
  families pass** (48 prior + 60 new: 12 per family across
  `readiness`, `gate-result`, `job-outcome`, `release-evidence`,
  `capability`).
* `cargo fmt --all -- --check` → clean.
* `cargo clippy --workspace --all-targets -- -D warnings` → clean.
* `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok.
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → passed.
* `openspec validate --specs --strict --no-interactive` → 3/3 passed
  (`rust-platform-foundation` now 4 requirements / 7 scenarios).

Evidence boundary for this change: the walkers prove the Rust validator
agrees with the language-neutral one on all 9 roadmap families, and the
typed-parity tests prove the new builders emit wire-valid documents.
`platform.public-portfolio-manifest` (tenth registry family, Forge/Hugo
owner, major version 1) is explicitly out of scope. No consumer adoption
is claimed. The pre-existing `.project.json` modification and the
`mailing-provider-adapters` / `post-mvp-readiness` proposals stay out of
this change's commits.

## Evidence — post-mvp-readiness (recorded 2026-09-30)

The change is implemented, archived to
`openspec/changes/archive/2026-09-30-post-mvp-readiness/`, and
spec-synced into `openspec/specs/readiness/spec.md` (5 requirements /
11 scenarios, plus a `## Purpose` section added at sync time because
strict spec validation requires it). Implementation commit is
`4ca307b`. It was selected as the next active change ahead of
`mailing-provider-adapters` because it is docs-only and touches no
shared arch-test/CI surface, so the mailing change lands later
without a race.

What was added (no `Cargo.toml`/`Cargo.lock`/`src/` edit, no new
dependencies, no consumer or sibling edit):

* `LICENSE` — verbatim Apache-2.0 text matching the workspace
  `license = "Apache-2.0"` declaration (copied from the registry
  copy of `serde`'s `LICENSE-APACHE`, not hand-typed).
* `README.md` — per-crate matrix (purpose, production/test-only
  posture, version, license, publish, MSRV, edition; every value
  restated from the workspace manifests), usage pointer, license
  pointer, and a path-dependency adoption guide that points to the
  `rust-platform-consumer-baseline` spec/recipe instead of
  re-declaring it.
* `docs/usage.md` — one minimal example per production crate
  (`platform-core`, `platform-contracts`, `platform-identity`,
  `platform-audit`); every snippet is byte-identical to its
  compiling `crates/*/examples/` file (verified by script, not by
  eye).
* `crates/platform-{core,contracts,identity,audit}/examples/*.rs`
  — compile-checked examples using only each crate's existing
  public API and existing dependencies (`cargo test --workspace`
  builds them; all four were also executed and ran clean).
  Test-only crates (`platform-testing`, `platform-arch-tests`) get
  no examples per the design decision.

Verification outputs (all fresh this session):

* `cargo test --workspace` → **104 passed, 0 failed** (count
  unchanged: examples build and run but add no test targets;
  `platform-arch-tests` green, so no example pulled a forbidden
  dependency into a production crate).
* `cargo build --examples --workspace` → clean; all four examples
  executed directly (`basics`, `envelope`, `subject_permission`,
  `audit_event`) → ran OK.
* `cargo fmt --all -- --check` → clean.
* `cargo clippy --workspace --all-targets -- -D warnings` → clean.
* `cargo deny check` → advisories ok, bans ok, licenses ok,
  sources ok (one standing `license-not-encountered` warning for an
  unmatched allowlist entry; warn, not deny).
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → 1/1
  passed (`mailing-provider-adapters` remains proposed).
* `openspec validate --specs --strict --no-interactive` → 4/4
  passed (new `spec/readiness` plus the 3 prior).
* `git diff --check` → clean.
* `git status` boundary: only `README.md`, `docs/`, `LICENSE`,
  `crates/*/examples/`, and this change's own
  `openspec/` paths appear. `Cargo.toml`, `Cargo.lock`,
  `crates/*/src/**`, `platform-testing`, `platform-arch-tests`,
  and every sibling repository are untouched. (The pre-existing
  `.project.json` working-tree modification stays out of this
  change's commits per the task caveats, as before.)

Evidence boundary for this change: the matrix, examples, and
license claim are proven consistent with the manifests and the
compiler today. Nothing here proves registry publication
(`publish = false` stays), consumer adoption beyond the
path-dependency recipe, or that the snippets stay compiling under
a future API — that is what `cargo test --workspace` guards on
every run.

## Evidence — mailing-provider-adapters (recorded 2026-09-30)

The change is implemented, archived to
`openspec/changes/archive/2026-09-30-mailing-provider-adapters/`,
and spec-synced into `openspec/specs/platform-mailing/spec.md` (4
requirements / 10 scenarios, plus a `## Purpose` section added at
sync time because strict spec validation requires it, as with
`readiness`). It was the last open proposal: `openspec/changes/`
now holds only `archive/`.

What was added (no `platform-core|contracts|identity|audit|testing`
source or public-API change; no sibling edit; `publish = false`
unchanged):

* `crates/platform-mailing/` — `Cargo.toml` (default features
  empty; `mock` and `smtp-lettre` opt-in; lettre/tokio optional),
  `src/lib.rs`, `src/types.rs` (`EmailAddress` display/bare parse
  with 254-byte / control-char / empty-local / whitespace limits,
  `EmailMessage` builder enforcing 1..=50 `to`, ≤50 total
  recipients, ≤998-char CR/LF-free subject, text-and/or-html `Body`
  ≤5 MiB, `AttachmentRef` by reference), `src/error.rs`
  (`MailingError` 7 variants, total `category()` match onto the
  frozen `ErrorCategory` with no wildcard arm, `to_safe_error()`,
  `scrub_addresses` redaction), `src/transport.rs`
  (`MailTransport` via async-trait for `dyn` on 1.75,
  `SendContext` with `CorrelationId` + `&dyn Clock`,
  `DeliveryReceipt` with clock-stamped `SentAt`),
  `src/mock.rs` (feature `mock`: scripted accept/fail outcomes,
  `Mutex` script, default-accept, timestamps only from the
  injected clock), `src/smtp.rs` (feature `smtp-lettre`:
  `SmtpTransportConfig` with loopback-only `Security::None`,
  `Secret`-wrapped credentials, 30 s default timeout;
  `LettreSmtpTransport` over `AsyncSmtpTransport<Tokio1Executor>`
  with rustls only; Bcc in envelope, stripped from wire headers by
  lettre's default `drop_bcc`), `tests/mail_mocks.rs`,
  `tests/mail_error_mapping.rs` (incl. the manifest-shape test
  proving lettre/tokio optional and default features empty),
  `examples/send_via_stub.rs` (default-features-clean stub send,
  executed OK).
* `crates/platform-arch-tests/` — the rule is now the pure
  function `check_crate_policy(crate, transitive-graph)` with the
  named-satellite table `SATELLITE_ALLOWLIST`
  (`platform-mailing → lettre, tokio`); `lettre` joined
  `FORBIDDEN_DEPS`/`DIRECT_FORBIDDEN_DEPS` for the four foundation
  crates; new unit tests for the allowlist pass, the synthesized
  `platform-identity`+lettre violation, and the unknown-satellite
  denial; the integration test applies the same pure rule to the
  real lockfile for foundation + satellite and asserts only the
  allowlisted satellite declares lettre.
* Workspace `Cargo.toml` — member registration plus
  `async-trait 0.1`, `tokio 1` (`net`, `time`, plus `rt`,
  `macros` for test/example runtimes), `lettre 0.11`
  (`builder`, `smtp-transport`, `tokio1`, `tokio1-rustls`,
  `rustls-tls`; no native-tls).
* `README.md` (mailing matrix row, satellite arch-test wording,
  deferred list now names HTTP mail APIs as the remaining wait),
  `ROADMAP.md` (stale "two changes done" header rewritten, step
  4c recorded), `docs/usage.md` + example (byte-identical,
  script-verified).

Verification outputs (all fresh this session):

* `cargo test --workspace` → **123 passed, 0 failed** (104 prior
  + 11 mailing lib + 4 mapping + 3 arch unit + 1 arch
  integration).
* `cargo test -p platform-mailing --no-default-features` → 15
  passed (11 + 4); `--features smtp-lettre,mock` → 23 passed
  (17 + 4 + 2).
* `cargo tree -p platform-mailing --no-default-features
  --edges normal` shows no lettre/tokio edge; with
  `smtp-lettre,mock` both resolve (lettre 0.11.23, tokio 1.53.1).
* Refused-connection probe (127.0.0.1 closed port, no external
  network) returns `TransportUnavailable` through the real lettre
  error path.
* `cargo fmt --all -- --check` → clean; `git diff --check` →
  clean.
* `cargo clippy --workspace --all-targets -- -D warnings` →
  clean (one rename: builder `to_many` → `extend_to` for
  `wrong_self_convention`).
* `cargo deny check` → advisories ok, bans ok, licenses ok,
  sources ok (standing unmatched-allowlist warnings only).
* `./scripts/conformance.sh` → all proofs hold.
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → no
  active changes remain (archive complete).
* `openspec validate --specs --strict --no-interactive` → 5/5
  passed (new `platform-mailing`: 4 requirements / 10
  scenarios).
* `cargo build --examples --workspace` → clean; all five
  examples executed directly (4 prior + `send_via_stub`) → ran
  OK. `platform-arch-tests` green, so no new dependency leaked
  into a foundation crate.

Design-ledger deviations recorded (all compile-needs, no scope
change): lettre needs the `builder` feature (message rendering)
and the `tokio1-rustls` bridge (lettre compile-errors on
`tokio1`+`rustls` without it) beyond the proposal's three
features; workspace tokio gains `rt`+`macros` (async tests and
the example need a runtime); `lettre::transport::smtp::ErrorKind`
does not exist as a public type (`Kind` is `pub(crate)` with
predicate accessors only), so the total mapping is factored as
`classify()` + `from_class()` over `SmtpFailureClass::ALL`
(6 classes, exhaustive match, enumerated test) instead of an
enum match — the mapping table in design §5 is otherwise
unchanged and `ErrorCategory` stays frozen per L3.

Evidence boundary for this change: the refused-connection probe
proves the unavailable branch against a live (local) failure;
the other five failure classes are proven through the
exhaustive classify/map unit test, not against live servers. No
consumer has migrated (alltools, rust-ecommerce, openpanel keep
their own plumbing until their own OpenSpec changes), and no
real SMTP delivery is claimed.

## Evidence — ai-provider-adapters (recorded 2026-09-30)

The change is implemented, archived to
`openspec/changes/archive/2026-09-30-ai-provider-adapters/`, and
spec-synced into `openspec/specs/platform-ai/spec.md` (6
requirements / 11 scenarios, plus a `## Purpose` section added at
sync time because strict spec validation requires it, as with the
two prior satellites). It was the last open proposal:
`openspec/changes/` now holds only `archive/`. Implementation
commit is `b1af193`.

What was added (no `platform-core|contracts|identity|audit|testing|mailing`
source or public-API change; no sibling edit; `publish = false`
unchanged):

* `crates/platform-ai/` — `Cargo.toml` (default features empty;
  `mock`, `openai-compat`, `ollama` opt-in; `reqwest`/`tokio`
  optional and feature-gated to the two transport features;
  `futures`/`async-stream` are contract deps), `src/lib.rs`,
  `src/types.rs` (`Role`, `Content`, `Message`, `GenerationOptions`,
  `ToolDefinition`, `ToolCall`, `FinishReason`, `Usage`,
  `ChatRequest` + builder, `ChatResponse`, `StreamChunk` with
  exactly-one final `Turn`, `Embedding`, `Capabilities` hand-rolled
  `u8` bit-set with `STREAMING|TOOLS|STRUCTURED|EMBEDDINGS`,
  `ProviderKind`, `ProviderInfo`, validation enforced at every
  transport entry), `src/error.rs` (`AiError` 10 variants, total
  `category()` match onto the frozen `ErrorCategory` with no
  wildcard arm, `to_safe_error()`, `truncate_safe_detail` Cow
  helper that caps at 200 bytes and strips control chars;
  `Display` redaction-audited: no API keys, prompts, completions,
  or raw response bodies), `src/provider.rs` (`AiProvider` and
  `EmbeddingProvider` via async-trait for `dyn` on 1.75, default
  trait methods refuse undeclared capabilities with
  `AiError::Unsupported`, declared-but-unimplemented with
  `AiError::Configuration`, `AiContext` with `CorrelationId` +
  `&dyn Clock`, `validate_embedding_batch` public helper),
  `src/mock.rs` (feature `mock`: scripted `ScriptedChat` /
  `ScriptedStream` / `ScriptedEmbed` outcomes, `Mutex` script,
  default Ok / zero-vector fallbacks, capability-gated stream
  refusal), `src/openai_compat.rs` (feature `openai-compat`:
  `OpenAiCompatConfig` with `Secret`-wrapped bearer, `base_url`
  loopback-only-http rule, `chat_path`/`embed_path` overrides,
  default 60 s timeout, default 8 MiB stream cap;
  `OpenAiCompatProvider` with `/v1/chat/completions`,
  `/v1/embeddings`, SSE streaming, JSON-schema structured output
  via `response_format: { type: "json_schema", json_schema:
  { strict, schema } }`, tool-call arguments JSON-parsed, content
  refusal detected from `error.message` containing
  `content_refused` and surfaced as `ContentPolicy` ahead of
  `from_status`; rustls only, no native-tls; `embed_with_model`
  separate method because the model id is not on the embed path),
  `src/ollama.rs` (feature `ollama`: `OllamaConfig` with default
  loopback endpoint, no credentials; `OllamaProvider` over
  `/api/chat` NDJSON and `/api/embed`, `"format": <schema>` for
  structured output; embedding count mismatch is
  `MalformedResponse`; same embedding-batch validation as
  openai-compat).
* `tests/ai_error_mapping.rs` (7 cases: invalid-message shapes,
  every-variant mapping table, `Display` redaction with no
  secret/prompt leak, boundary redaction, manifest-shape proving
  `reqwest`/`tokio` optional and default features empty,
  `unsafe_code` forbid sentinel, `FinishReason` snake-case
  stability), `tests/ai_mock.rs` (8 cases: scripted sequence,
  capability gating, stream deltas + terminal Turn, stream
  error, embed scripted/zero/empty/oversize batch),
  `tests/ai_openai_compat.rs` (12 wiremock cases: happy chat
  round-trip, tool-call args JSON-parse, SSE deltas + terminal
  Turn, structured `json_schema` body, embeddings batch, 401 →
  `Unauthorized`, 429 → `RateLimited`, 500 with `content_refused`
  body → `ContentPolicy`, 500 without → `ProviderServer`,
  malformed JSON → `MalformedResponse`, stream truncation
  without `[DONE]` → `MalformedResponse`, empty embed model →
  `Validation` before any socket), `tests/ai_ollama.rs` (11
  wiremock cases: happy chat, NDJSON deltas + terminal Turn,
  truncation → `MalformedResponse`, mid-stream server `error` →
  `ProviderServer`, 404 → `Validation`, 500 → `ProviderServer`,
  embeddings batch, count mismatch → `MalformedResponse`,
  structured `"format"` field, empty model and empty batch
  refused pre-socket), `examples/ai_basics.rs` (default-features-
  clean stub chat, executed OK).
* `crates/platform-arch-tests/` — `SATELLITE_ALLOWLIST` now holds
  `("platform-ai", &["reqwest", "tokio", "futures"])` alongside
  the existing `("platform-mailing", &["lettre", "tokio"])`. The
  `only_allowlisted_satellites_directly_depend_on_lettre`
  integration test now checks "uses at least one allowlisted
  leaf" instead of "uses lettre" so the new satellite's distinct
  leaves don't break the vacuous-pass guard. New unit test
  `ai_satellite_allowlist_permits_reqwest` exercises both the
  pass path (the satellite's graph) and the failure path
  (overreach with `lettre`); new unit test
  `foundation_crate_cannot_reach_reqwest` synthesizes a
  foundation-graph with `reqwest` and asserts the violation. New
  integration test `ai_satellite_actually_depends_on_its_allowlisted_leaves`
  is the per-allowlist vacuous-pass guard for `platform-ai`.
* Workspace `Cargo.toml` — member registration plus
  `reqwest 0.12` (`default-features = false`, features
  `["json", "stream", "rustls-tls", "http2"]`; rustls only, no
  native-tls), `futures 0.3`, `async-stream 0.3`. `wiremock 0.6`
  is a dev-dep of `platform-ai` only.
* `README.md` (per-crate matrix gains the `platform-ai` row,
  implemented-crates list gains its summary, deferred list
  explicitly keeps Anthropic out and names OpenAI-compatible +
  ollama as the shipped satellite), `ROADMAP.md` (stale "six
  changes done" header rewritten, step 4d recorded with the
  two-consumer extraction decision), `docs/usage.md` + example
  (byte-identical, `cargo test --workspace` builds it).

Verification outputs (all fresh this session):

* `cargo test --workspace` → **193 passed, 0 failed** (123 prior
  + 32 platform-ai lib + 7 error-mapping + 8 mock + 11 ollama +
  12 openai-compat). The mailing count is unchanged.
* `cargo test -p platform-ai --no-default-features` → 23 passed
  (16 lib + 7 error-mapping). `cargo test -p platform-ai
  --features mock,openai-compat,ollama` → 70 passed (32 lib + 38
  integration). `cargo test -p platform-arch-tests` → 14 passed
  (8 lib + 6 integration; the new `ai_satellite_*` cases and the
  new vacuous-pass guard included).
* `cargo tree -p platform-ai --no-default-features --edges normal`
  shows no `reqwest`/`tokio` edge; with `--features
  mock,openai-compat,ollama` both resolve (reqwest 0.12.x, tokio
  1.53.1) along with the full transitive graph (hyper, h2,
  rustls, etc.).
* `cargo fmt --all -- --check` → clean; `git diff --check` →
  clean.
* `cargo clippy --workspace --all-targets -- -D warnings` →
  clean.
* `cargo deny check` → advisories ok, bans ok, licenses ok,
  sources ok (standing unmatched-allowlist warnings only).
* `./scripts/conformance.sh` → all proofs hold (no
  foundation/production-crate change, so all six proofs
  carry over).
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → no
  active changes remain (archive complete).
* `openspec validate --specs --strict --no-interactive` → 6/6
  passed (new `platform-ai`: 6 requirements / 11 scenarios).
* `cargo build --examples --workspace` → clean; all six
  examples executed directly (5 prior + `ai_basics`) → ran OK.
  `platform-arch-tests` green, so no new dependency leaked
  into a foundation crate.

Design-ledger deviations recorded (all compile-needs, no scope
change): the SSE parser initially emitted a duplicate `Turn`
when `[DONE]` followed by an empty body completed the stream
(fixed in commit `b1af193` by clearing `last_finish` on
`[DONE]`, since the post-loop fallback is meant for
stream-ended-without-`[DONE]` only); the source's
`skip_serializing_if = "std::ops::Not::not"` on the `stream: bool`
field omits `stream: false` for non-streaming requests (which
matches the OpenAI 06-2025 wire shape, where `stream` defaults
to `false` when absent, so no behavior changes), and the
`role_wire_form_is_snake_case` unit test was moved into the
test module's `use` to silence the `unused_imports` lint.

Evidence boundary for this change: the wiremock fixtures
replay canned responses through the real `reqwest` and
`async-stream` parsers, so the SSE / NDJSON byte handling,
status-code mapping, content-refusal detection, embedding-batch
count validation, and `truncate_safe_detail` redaction are
proven against the same code path that production would call
— but no live provider is contacted (CI has no API keys, and
the design is explicit that wiremock is the only test
fixture). No consumer has migrated (alltools keeps its own
plumbing until its own OpenSpec change), and no live AI
call is claimed. The pre-existing `.project.json` working-
tree modification stays out of this change's commits per
the task caveats, as before.

## Evidence — registry-publication (recorded 2026-09-30, archived 2026-10-01)

The change is implemented, archived to
`openspec/changes/archive/2026-09-30-registry-publication/`, and
spec-synced into `openspec/specs/rust-platform-publication/spec.md`
(4 requirements / 8 scenarios, plus a `## Purpose` section added at
sync time). The pipeline is shipped; the actual `cargo publish`
release stays BLOCKED on P6 (real GitHub-publication precondition
plus the owner-added `PUBLICATION-APPROVED` marker), with the exact
owner actions recorded in `docs/publication.md` §Pre-publish
checklist. Implementation commit is `8e43e7d`. It was selected as
the next active change because the only remaining work after the
seven foundation / payload / satellite changes was the
publication-pipeline metadata and workflow, with no new crate,
trait, or contract surface.

What was added (no foundation / satellite source or public-API
change; no sibling edit; `publish = false` for the workspace-internal
crates explicitly kept):

* `[workspace.package]` in root `Cargo.toml` — `publish = true`,
  `repository` / `homepage` (a documented placeholder URL that the
  owner replaces when the GitHub-publication precondition lands),
  `readme = "README.md"`, `keywords = ["platform", "rust",
  "contracts", "cross-cutting", "framework-neutral"]`,
  `categories = ["development-tools"]`. Per-crate
  `documentation = "https://docs.rs/<name>"` and
  per-crate `keywords` / `categories` / `exclude` on every
  publishable manifest. `platform-arch-tests` keeps an explicit
  `publish = false` override so workspace inheritance cannot
  accidentally re-enable publication. `tests/consumer-fixture`
  also keeps an explicit `publish = false` override so the
  path-dependency conformance fixture stays workspace-internal.
* Inter-crate deps in the registry-addressable
  `{ path = "../platform-core", version = "0.1.0" }` form on every
  publishable crate manifest so `cargo publish` rewrites path deps
  to version requirements at upload time.
* `.publish-order` — the topological publish order
  (`platform-core` → `platform-contracts` → `platform-identity` →
  `platform-audit` → `platform-testing` → `platform-mailing` →
  `platform-ai`), one crate per line, comments above each entry
  naming the reason. Read by `.github/workflows/publish.yml` and
  asserted against the manifest DAG by a unit test in
  `platform-arch-tests` (see below).
* `.github/workflows/publish.yml` — tag-gated (`v[0-9]+.[0-9]+.[0-9]+*`
  only; branch pushes never publish), with explicit
  `permissions: contents: read`. Steps in order:
  - `Refuse without PUBLICATION-APPROVED marker` — design P3 / P6
    gate; failure prints the precondition message and exits
    before any registry call.
  - `Refuse without CARGO_REGISTRY_TOKEN` — design P3 secret gate.
  - `Verify tag version matches workspace version` — reads
    `GITHUB_REF` on push, the most recent `v*` tag on dispatch,
    compares against `sed -n 's/^version *= *"\([^"]*\)".*/\1/p'
    Cargo.toml`; failure names both values and exits before any
    publish step runs.
  - `Read publish order` — parses `.publish-order` (comment and
    blank lines stripped) into a `GITHUB_OUTPUT` for downstream
    steps.
  - `Verify name availability (best-effort pre-flight)` — 404
    check on `https://crates.io/api/v1/crates/<name>` for every
    name; non-404 fails the job (re-checking first-come names).
  - `Install Rust toolchain` (dtolnay/rust-toolchain@stable,
    components `rustfmt` + `clippy`) and `Cache cargo registry`
    (Swatinem/rust-cache@v2).
  - `Publish crates in topological order` — per-crate
    `cargo publish -p "$crate"` (no `--allow-dirty`, no
    `--no-verify`; both are banned by design §0 / §6), with
    per-crate 404 pre-flight on the version, 30-attempt × 30 s
    propagation poll against
    `https://crates.io/api/v1/crates/<name>/<version>`, and
    skip-existing idempotence (a 200 on the pre-flight means
    "already published", so the crate is skipped and the loop
    continues; a non-200 / non-404 fails the job). `workflow_dispatch`
    requires the operator to type `approve` so a manual retry
    never bypasses the precondition.
* `crates/platform-arch-tests/src/lib.rs` — three new unit tests:
  - `publish_order_covers_every_publishable_crate_exactly_once` —
    every non-comment, non-blank line in `.publish-order` is a
    real workspace member; `platform-arch-tests` is never listed;
    every publishable crate appears exactly once.
  - `publish_order_respects_dependency_dag` — for every `b` in
    `.publish-order`, every `platform-*` direct dependency of `b`
    must appear earlier in the order; the failure message names
    the offending pair and their positions.
  - `arch_tests_manifest_explicitly_publish_false` — reads
    `crates/platform-arch-tests/Cargo.toml` verbatim and asserts
    `package.publish` is set to `false` explicitly, so a future
    change cannot accidentally re-enable publication through
    workspace inheritance.
* `docs/publication.md` — the policy document: lockstep
  versioning (one workspace version, bumped in one commit with a
  matching `vX.Y.Z` tag and a changelog entry), the pre-1.0
  semver rule (patch = compatible, minor = break), the yank
  procedure (manual `cargo yank` per crate, every yank recorded
  in the changelog with reason and replacement), the
  `CARGO_REGISTRY_TOKEN` recipe (crate-scoped, time-limited,
  rotated on maintainer change), and the two-precondition
  pre-publish checklist (real GitHub-publication URL via
  `scripts/github_metadata.py`; owner-added `PUBLICATION-APPROVED`
  marker). The changelog section starts with one `_pending_`
  row for the first `v0.1.0` release.
* `README.md` — header paragraph now names the
  `registry-publication` pipeline and points at the BLOCKED
  release; the crate-matrix trailer paragraph records the seven
  publishable crates + the explicit `publish = false` overrides
  on `platform-arch-tests` and `consumer-fixture`.
* `ROADMAP.md` — `4e. registry-publication — done (pipeline);
  release BLOCKED on P6 owner approval` records the change with
  the full dependency-order step; the deferred list keeps
  `axum-web-adapters` and `sqlx-session-and-audit-adapters` as
  their own future changes.
* `docs/usage.md` — a "Publishing" section at the end of the
  existing per-crate examples names the seven publishable crates,
  the workspace-internal overrides, and the two preconditions in
  `docs/publication.md` §Pre-publish checklist.

Verification outputs (pinned toolchain 1.95.0, all fresh this
session):

* `cargo publish --dry-run -p <crate>` for all seven publishable
  crates succeeds (with a one-time local `[patch.crates-io]` in a
  gitignored `.cargo/config.toml` that redirects the
  `platform-*` deps to local paths, simulating the
  "platform-core is already on the registry" state for the
  dependent crates; the real workflow runs the topological order
  with propagation polls so the registry form is real at each
  step). Output (abridged):
  * `platform-core`: `Packaged 13 files, 36.9KiB (11.1KiB compressed)`;
    `Verifying platform-core v0.1.0` recompiles the packaged
    source and finishes clean; `aborting upload due to dry run`.
  * `platform-contracts`: `Packaged 21 files, 99.4KiB (24.5KiB
    compressed)`; clean.
  * `platform-identity`: `Packaged 17 files, 73.3KiB (19.2KiB
    compressed)`; clean.
  * `platform-audit`: `Packaged 11 files, 56.9KiB (15.2KiB
    compressed)`; clean.
  * `platform-testing`: `Packaged 11 files, 47.0KiB (13.2KiB
    compressed)`; clean.
  * `platform-mailing`: `Packaged 13 files, 94.9KiB (25.1KiB
    compressed)`; clean (the pre-existing `yoke-derive` yanked
    warning is unchanged from prior changes).
  * `platform-ai`: `Packaged 16 files, 216.6KiB (47.1KiB
    compressed)`; clean.
  * `platform-arch-tests`: refused with
    ``error: `platform-arch-tests` cannot be published.
    `package.publish` must be set to `true` or a non-empty list
    in Cargo.toml to publish.``
  * `consumer-fixture`: refused with the same message after the
    new explicit `publish = false` override.
* `cargo package -p <crate> --no-verify --list` for all seven
  publishable crates excludes `openspec/` and `.github/` (zero
  leaks per crate, asserted with `grep -cE`).
* Bash dry-run of the workflow's precheck logic with the local
  fixtures (marker absent, tag `v0.99.0` ≠ workspace `v0.1.0`,
  `CARGO_REGISTRY_TOKEN` unset, order file present with 7
  crates) confirms each negative fails with the named reason and
  the positive (tag `v0.1.0` == workspace `v0.1.0`) proceeds —
  no real network calls, no real registry writes.
* YAML lint: `python3 -c "import yaml; …"` parses
  `.github/workflows/publish.yml` to a single `publish` job.
* `cargo test --workspace` → **152 passed, 0 failed** (149 prior
  + 3 new arch-tests). The 152 number is the default-features
  count; `cargo test --workspace --all-features` → 207 (the
  `mock` / `ollama` / `openai-compat` / `smtp-lettre` feature
  tests). The 41-test difference matches the platform-ai and
  platform-mailing feature-gated tests, all unchanged from prior
  changes.
* `cargo test -p platform-arch-tests` → 11 unit + 6 integration,
  all green; the three new tests
  (`publish_order_covers_every_publishable_crate_exactly_once`,
  `publish_order_respects_dependency_dag`,
  `arch_tests_manifest_explicitly_publish_false`) included.
* `cargo fmt --all -- --check` → clean.
* `cargo clippy --workspace --all-targets -- -D warnings` → clean.
* `cargo deny check` → advisories ok, bans ok, licenses ok,
  sources ok. Adding `publish = false` to `consumer-fixture` was
  required because the workspace-internal `..`-path deps on
  `../../crates/platform-*` are wildcard paths that
  `cargo-deny` refuses for crates it considers publishable.
* `./scripts/conformance.sh` → all proofs hold (the
  `consumer-fixture` path-dependency conformance harness still
  builds, tests, formats, and lints clean under the unchanged
  baseline).
* `node scripts/check-openspec-change-names.mjs` → ok.
* `openspec validate --changes --strict --no-interactive` → no
  active changes remain (archive complete).
* `openspec validate --specs --strict --no-interactive` → 7/7
  passed (new `rust-platform-publication`: 4 requirements / 8
  scenarios).
* `cargo build --examples --workspace` → clean; all six
  examples unchanged and still execute.
* `git diff --check` → clean.

Design-ledger deviations recorded (all compile-needs, no scope
change): `cargo-deny` introduced a new wildcard-paths refusal
once the workspace default flipped to `publish = true` (the
`tests/consumer-fixture` path deps on `../../crates/platform-*`
are wildcard paths that `cargo-deny` refuses for crates it
considers publishable); the fix is the explicit `publish = false`
override on `consumer-fixture` documented above. The
`yoke-derive v0.8.3` yanked warning surfaces again on
`platform-mailing` and `platform-ai` dry-runs (pre-existing
transitive pull-in via `icu_normalizer` / `icu_properties`,
unchanged by this change). The `[patch.crates-io]`
`.cargo/config.toml` used to run the local dry-runs in
topological order is gitignored (`.cargo/` is in `.gitignore`),
is not part of the change, and is recreated on demand by anyone
re-running the dry-run oracle.

Evidence boundary for this change: the workspace builds, tests,
formats, lints, denies, and validates clean with the new
metadata, the new workflow YAML, the new order file, the new
policy doc, and the three new arch-tests in place; the
topological `cargo publish --dry-run` for all seven publishable
crates succeeds (with the local-path patch simulating the
registry state), the package lists exclude the internal paths,
and `platform-arch-tests` and `consumer-fixture` correctly
refuse. No real registry call is made; no real `v0.1.0` tag is
pushed; the `PUBLICATION-APPROVED` marker is absent by design.
The release is BLOCKED on P6, and the change is only "DONE for
the implementation" — the actual `v0.1.0` release lands when the
owner (a) runs `scripts/github_metadata.py --publish` from
workspace-governance to publish the repository and set the real
`[workspace.package] repository` URL, (b) adds the
`PUBLICATION-APPROVED` marker file in a tracked commit, and
(c) pushes the `v0.1.0` tag. The result of (c) is recorded in
HANDOFF as release evidence at that time.

## Evidence boundary

The 152 workspace tests, the 108-case × 9-family
cross-repository fixture parity walk, the dependency-direction
lockfile oracle (foundation crates plus the named-satellite
allowlist), the three new `registry-publication` unit tests
that bind `.publish-order` to the manifest DAG, the
`cargo clippy` clean run, and the strict `openspec validate`
prove that the **wire-format and crate-shape** of the
foundation plus the second-layer permission / audit / tenant
helper surface plus the operational payloads plus the SMTP
mailing satellite plus the AI capability satellite plus the
crates.io publication pipeline is consistent,
machine-checkable, and provider-neutral. They do not prove that
any consumer (`openaccounting`, `rust-ecommerce`, `hookit`,
`openpanel`, `vowra`, or a future Rust service) has adopted
these crates in production; that evidence will come from each
consumer's own OpenSpec change.

In particular:

* All 9 roadmap contract families are exercised in the in-tree
  parity walker. `platform.public-portfolio-manifest` (tenth
  registry family, Forge/Hugo owner, major version 1) stays out of
  scope.
* The fixture parity proves the Rust validator agrees with the
  language-neutral one on documents that already conform to the
  schema. It does not prove that the Rust types are semantically
  faithful to every consumer's existing identity, permission, or
  audit logic; that is the application-owned side of the boundary
  described in `crates/platform-identity/src/lib.rs`.
* The dependency-direction test forbids `axum`, `sqlx`, `cookie`,
  `jsonwebtoken`, `openidconnect`, `oauth2`, `reqwest`, `lettre`,
  `tokio`, and any product crate in the four foundation crates,
  while the named-satellite allowlist grants `lettre`/`tokio` to
  `platform-mailing` alone. It does not assert that the
  application that adopts the foundation is itself provider-
  neutral; applications may pull in any framework they want behind
  the platform's trait surface.
* The `AllowListEvaluator` is the smallest non-trivial reference
  implementation. The platform deliberately does not ship an
  RBAC DSL, role inheritance, or attribute-based policy language;
  applications plug in their real policy behind the
  `PermissionEvaluator` trait. The reference impls are safe
  defaults and tests fixtures, not a recommended production
  policy.
* The `registry-publication` pipeline is implemented and
  machine-checked; the actual `cargo publish` release is BLOCKED
  on P6 (real GitHub-publication precondition + owner approval).
  No registry call is made until the owner pushes the
  `v0.1.0` tag with the `PUBLICATION-APPROVED` marker tracked.

## Next action

All OpenSpec changes are implemented, archived, and spec-synced
(`openspec/specs/`: `rust-platform-foundation`,
`rust-identity-audit-permission`, `rust-platform-consumer-baseline`,
`readiness`, `platform-mailing`, `platform-ai`,
`rust-platform-publication`; `openspec/changes/` holds only
`archive/`). The remaining work is the real crates.io release
(visible in `docs/publication.md` §Pre-publish checklist) followed
by per-consumer adoption changes (alltools, rust-ecommerce,
openpanel) in their own repositories. Until the release lands the
foundation plus the second-layer permission / audit / tenant
helpers plus the full operational payload surface plus the
readiness docs plus the SMTP mailing satellite plus the AI
capability satellite plus the crates.io publication pipeline
are the working state.
