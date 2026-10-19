# Design: platform-mailing crate and satellite dependency policy

## 1. Implementation boundary

- Repository: `rust-platform-libs` only.
- New crate: `crates/platform-mailing/` — `Cargo.toml`, `src/lib.rs`,
  `src/types.rs` (message model), `src/error.rs`, `src/transport.rs`
  (trait), `src/mock.rs`, `src/smtp.rs`, `tests/mail_mocks.rs`,
  `tests/mail_error_mapping.rs`.
- Changed files: workspace `Cargo.toml` (member), `crates/platform-arch-tests/`
  (allowlist table + new assertions), `README.md`, `ROADMAP.md`,
  `HANDOFF.md` (docs steps at completion time).
- Must not change: `platform-core|contracts|identity|audit|testing` sources
  or public APIs; any sibling repository; CI workflow semantics beyond
  inheriting the existing `cargo test --workspace`.

## 2. Language and runtime

- Rust, edition 2021, `rust-version = "1.75"` (matches workspace today).
- Async: `async-trait` (object-safe `dyn MailTransport` for consumer
  injection; RPITIT rejected for dyn-compat at 1.75).
- Adapter deps: `lettre = { version = "0.11", default-features = false,
  features = ["smtp-transport", "tokio1", "rustls-tls"] }` — mirrors both
  lettre-0.11 consumers (alltools uses rustls; native-tls is banned so the
  transitive tree stays OpenSSL-free), `tokio = { version = "1", features =
  ["net", "time"] }` (connect/timeout control), `thiserror = "2"`? — No:
  error model hand-rolled onto `platform_core::SafeError`/`ErrorCategory`
  to match foundation convention; `base64` etc. only if a compile need
  arises, recorded in the ledger.
- Build/test commands: `cargo test --workspace`;
  `cargo test -p platform-mailing --no-default-features`;
  `cargo test -p platform-mailing --features smtp-lettre,mock`;
  `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Target platforms: Linux/macOS dev matrices as today; no Windows promise.

## 3. Ownership and shared code

- Decision from reconnaissance: **extend shared owner** — the capability is
  recurring across three consumer repos, and the shared home is
  rust-platform-libs, whose README already reserves satellite adapters for
  exactly this moment.
- Dependency direction: consumers → `platform-mailing` → `platform-core`
  (+ optional lettre/tokio under the feature). Foundation crates → nothing
  new. `platform-mailing` MUST NOT depend on `platform-testing`
  (production-crate rule stands); its mock is an in-crate `mock` feature.
- Version/release boundary: crate version = workspace `0.1.0`,
  `publish = false` retained (registry = roadmap decision D4); consumers
  adopt by git/path dependency under the same recipe as `trailCrew`.
- Adoption steps are documented, not performed here (each consumer's own
  OpenSpec change; roadmap B1 adoption rows).

## 4. Behavioral model

Inputs/outputs only — no state, no persistence, no retries.

| Element | Definition |
|---|---|
| `EmailAddress` | parsed `display-name <local@domain>`; rejects empty local, no `@`, control chars, >254 bytes total |
| `EmailMessage` | from (1), to (≥1, ≤ 50), cc/bcc optional, `subject` (≤ 998 chars, no CR/LF), one `Body` (text and/or html, ≤ 5 MiB total declared size), attachments by reference (`Vec<AttachmentRef>` — name + opaque `Bytes`; loading is caller-owned) |
| `MailTransport` (trait) | `async fn send(&self, msg: &EmailMessage, cx: &SendContext) -> Result<DeliveryReceipt, MailingError>` |
| `SendContext` | carries `CorrelationId` (platform-core) and `&dyn Clock` |
| `DeliveryReceipt` | provider message id as `Option<bounded String>` + `SentAt` timestamp from the injected clock; no per-recipient receipt (SMTP has none) |
| Invariants | validation runs BEFORE transport; a returned `Ok` means the server accepted the message for relay, not delivered; idempotency/dedupe/queue are caller concerns; `Bcc` MUST be stripped from wire recipients copy while counted in envelope |

Concurrency: `&self` send (implementations own interior mutability); no
ordering guarantees across calls. No auth/tenant model — the platform is
credential-blind; SMTP auth lives in the adapter's config type.

## 5. Contract and compatibility

- Public surface (all re-exported from `lib.rs`): `EmailAddress`,
  `EmailMessage`, `Body`, `AttachmentRef`, `SendContext`, `DeliveryReceipt`,
  `MailingError`, `MailTransport`, `mock::MockTransport` (feature `mock`),
  `smtp::SmtpTransportConfig` + `smtp::LettreSmtpTransport` (feature
  `smtp-lettre`).
- `SmtpTransportConfig`: host (`bounded str`), port (u16, default 587),
  `Security` enum { `StartTlsOrUpgrade`, `Tls`, `None` — `None` accepted only
  for loopback hosts, validated }, `Secret<String>` username/password,
  timeout `Duration` (default 30 s), `pool: bool` left to lettre default.
  Env-key convention for consumers: `<APP>_SMTP_*` documented, not enforced.
- `MailingError` variants map 1:1 onto `platform_core::ErrorCategory`:
  `Validation(String)` → Invalid; `TransportUnavailable(reason)` →
  Unavailable; `RejectedByServer(smtp code + safe reason)` → the closest
  existing `ErrorCategory` variant shipped by `platform-core`; if none fits
  cleanly the category is `Unavailable` carrying the server code in the safe
  detail. The enum stays frozen for this change (ledger L3): implementation
  MUST NOT add `ErrorCategory` variants. `Authentication` →
  Unavailable-with-auth-kind; `Timeout` → Unavailable; `Tls` → Unavailable;
  `Configuration(String)` → Invalid. All errors `SafeError`-compliant:
  `Display` never contains credentials, message bodies, or full addresses
  (local-part redacted to domain).
- Serde: none in v1 (code contract, not wire contract; no platform-contracts
  schema family involved).
- Backward compatibility: brand-new crate; future breaking changes follow
  the repo compatibility policy (semver + OpenSpec change).

## 6. Failure and boundary policy

| Case | Behavior |
|---|---|
| Malformed address/message | `Err(Validation)`, transport never invoked |
| >50 recipients or oversize body | `Err(Validation)` |
| Connect refused/DNS error | `Err(TransportUnavailable)`; NO retry |
| Connect/send exceeds `timeout` | `Err(Timeout)`; tokio time under feature only |
| AUTH rejected (535) | `Err(Authentication)`; credentials redacted |
| Server rejects data (5xx) | `Err(RejectedByServer)` carrying code + server string |
| TLS negotiation fails | `Err(Tls)` |
| `Security::None` with non-loopback host | `Err(Configuration)` at construction |
| Feature `smtp-lettre` off, smtp symbols used | compile error (module gated) |
| Default features | crate compiles and tests pass with lettre absent |
| Empty `to` | `Err(Validation)` |

No partial success: one message, one result. Rollback is N/A (no state).

## 7. Verification oracle

- Unit (in-crate, no network): address/message validation success/failure/
  boundary (254-byte address, 5 MiB body, 51st recipient, CR/LF subject);
  redaction tests assert `Display` of every `MailingError` variant built with
  secret material omits it (`assert_redacted!`-style local test, not
  platform-testing crate).
- Mock transport tests: scripted success/failure/timeout outcomes; clock
  injection produces deterministic `SentAt`.
- Adapter mapping tests: `lettre::transport::smtp::Error` and `ErrorKind`
  cases (constructed, not live) map to the declared `MailingError` variants —
  total-mapping test enumerates every lettre error kind.
- Feature matrix: the three build commands in §2 all green; a test asserts
  `cargo tree -p platform-mailing --no-default-features` contains no lettre/
  tokio edges.
- Architecture: extended `platform-arch-tests` — (a) foundation four remain
  free of lettre/tokio/reqwest/axum/sqlx/…; (b) ONLY `platform-mailing` may
  reach lettre (allowlist by crate name); (c) no production crate depends on
  `platform-mailing` unless it is a satellite leaf (foundation unchanged);
  (d) `platform-testing` ban unchanged. Negative case: the rule check is a
  pure function over a dependency graph, so a unit test feeds it a
  synthesized graph with lettre attached to `platform-identity` and asserts
  failure — the real lockfile walk alone cannot exercise a violation that
  does not exist.
- Evidence before any task box may tick: command + exit summary recorded in
  HANDOFF at completion; `openspec validate --changes --strict
  --no-interactive` green; strict specs validation after archive.

## 8. Decision ledger

| # | Decision | Choice | Alternatives rejected |
|---|---|---|---|
| L1 | Crate home | satellite in rust-platform-libs | new workspace repo (fragments ownership); per-consumer duplication (the status quo) |
| L2 | Async model | async-trait dyn-injectable | sync-only (consumers are async); RPITIT (breaks `dyn`) |
| L3 | Error taxonomy | reuse existing `ErrorCategory` variants; `RejectedByServer` gets the closest existing category, enum stays frozen | extending platform-core enum (violates shipped spec without its own change) |
| L4 | Scope | SMTP + trait + mock | include Resend/Brevo (one consumer — rule says wait); include router/quota (same) |
| L5 | Features | `smtp-lettre`, `mock`, default none | default-on lettre (violates opt-in philosophy) |
| L6 | Arch policy | named-satellite allowlist table | weakening the global ban (breaks foundation spec); hard-coding exceptions in consumers |
| L7 | TLS | rustls only | native-tls (OpenSSL variance across consumers) |
| L8 | Ordering | implementation may not start while `rust-platform-consumer-baseline`/`post-mvp-readiness` hold the repo's selection rules; owner picks sequence in HANDOFF before work | silently editing shared arch-test files in parallel |
| — | Blockers | none; D4 (publication) does not gate this crate | — |

## Traceability

| Req | Proposal outcome | Design section | Boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| MAIL-001 typed send contract | What Changes #1 | §4 §5 §6 | types.rs, error.rs, transport.rs | success/failure/boundary | 2.1–2.3 | §7 unit + mapping tests |
| MAIL-002 opt-in SMTP adapter | What Changes #1 | §2 §5 §6 | smtp.rs + feature | success/failure/boundary | 2.4 | §7 feature matrix + mapping tests |
| MAIL-003 deterministic mock | What Changes #1 | §4 §6 | mock.rs + feature | success/boundary | 2.5 | §7 mock tests |
| MAIL-004 dependency-direction policy | What Changes #2 | §3 §7 | platform-arch-tests | success/failure/boundary | 2.6 | §7 arch tests |
