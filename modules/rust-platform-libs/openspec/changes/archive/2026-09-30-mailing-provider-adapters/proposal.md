# Proposal: A shared mailing capability crate

## Why

Three Rust consumers already implement the same email-delivery seam twice
over: `alltools-platform` (`rust/packages/at-domain-email/` — lettre 0.11
SMTP plus provider failover), `rust-ecommerce` (`src/email/smtp.rs` — lettre
0.11), and `openpanel` (`crates/openpanel-app/src/mail/` +
`notifications/adapter.rs` — `AsyncSmtpTransport` with relay/STARTLS). The
`ROADMAP.md` deferral policy states provider adapters land only after at
least two real consumers demonstrate a compatible contract and an explicit
OpenSpec change records the decision. This change is that recorded decision
for SMTP mail delivery: the contract is demonstrated by three consumers on
the same lettre major version.

## What Changes

- New opt-in crate `platform-mailing`: provider-neutral message/error types,
  a `MailTransport` send contract mapped onto `platform-core` error
  categories, a deterministic mock transport, and an SMTP adapter behind the
  `smtp-lettre` cargo feature.
- `platform-arch-tests` scope update: the forbidden-dependency rule keeps
  protecting the four foundation crates and re-exports nothing new; a named
  satellite crate (`platform-mailing`) gains a bounded allowlist
  (`lettre`, its transitive tokio) while the ban on any foundation crate or
  `platform-testing` dependency from production code is preserved.
- Workspace member registration, README/ROADMAP/HANDOFF status updates at
  implementation time.

## Package Boundary and Split Assessment

**Single outcome:** a consumer can send email through one typed, provider-
neutral Rust contract without duplicating lettre plumbing.

**Included:** message/error types, transport trait, mock, SMTP adapter,
arch-test allowlist, feature-gated builds.

**Excluded (split signals):** HTTP API provider adapters (Resend/Brevo)
have exactly one consumer (alltools) and stay deferred per the two-consumer
rule; router/quota/rate-limit logic moves only when a second consumer needs
it; DKIM signing and IMAP/webmail are openpanel-specific; template
rendering is a separate capability decision; consumer-side migration edits
are each consumer's own OpenSpec change.

**Dependencies:** `rust-platform-foundation` (shipped) supplies `ErrorCategory`,
`SafeError`, `Clock`. Registry publication (currently `publish = false`) is
decision D4 in `TOOLCHAIN_ROADMAP.md` and does not block this crate existing.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| alltools-platform | `rust/packages/at-domain-email/src/providers/{yandex,mock}.rs`, `domain/traits.rs` | ports-and-adapters mail layer, lettre 0.11 rustls SMTP, typed provider traits | depends on `at-infra-db`; product router/quota inside crate | alltools owns product; rust-platform-libs owns contract | **extract recurring capability** (contract + SMTP adapter + mock only) |
| rust-ecommerce | `src/email/smtp.rs` (lettre 0.11) | second independent lettre usage | bespoke message type; queue coupling | rust-ecommerce | **consumer evidence** (adopt later in own change) |
| openpanel | `crates/openpanel-app/src/mail/`, `notifications/adapter.rs` | third SMTP seam (relay/STARTTLS) | webmail/DKIM product features | openpanel | **consumer evidence**; DKIM/IMAP stay local |
| dotnet-platform-libs | `src/Platform.Mailing`, `Platform.Mailing.SendGrid` | established shape: contracts package + provider adapters + testing double | different ecosystem; mirror shape only | dotnet-platform-libs | **adapt through a generic adapter** (pattern) |
| this repository | `crates/platform-core` (`ErrorCategory`, `SafeError`), `platform-arch-tests` | error/clock vocabulary; dependency-direction oracle | arch rule today bans tokio/lettre from ALL production crates | rust-platform-libs | **extend shared owner** (satellite allowlist) |
| platform-contracts | schema families | none for mail (no wire contract needed) | N/A | platform-contracts | **not applicable** |

Extraction here names current consumers (alltools, rust-ecommerce,
openpanel), the contract (this crate's trait surface), and the migration
path (each consumer adopts in its own change, then deletes duplicated
plumbing). This package does not authorize editing any sibling repository.

## BFS Impact Map

- **Capabilities:** new `platform-mailing`; arch-test policy scope; README
  implemented/deferred lists; ROADMAP step 6-ish deferral note for mailing.
- **Unaffected:** the four foundation crates' public APIs, `platform-
  testing`, `platform-identity/audit/contracts` specs, CI beyond the existing
  `cargo test --workspace`, every sibling repo.
- **Callers:** none in-tree (satellite crates are leaf); consumers wire in
  later under their own changes.
- **Failure/boundary:** malformed address, unavailable host, rejected
  credentials, timeout — each must map to one typed `MailingError` category;
  no implicit retry, dedupe, or persistence in the platform.
- **Compatibility:** default features off keeps the crate lettre-free;
  foundation neutrality is restated, not weakened; `async-trait` used to
  preserve object safety on the pinned toolchain.
- **Security/privacy:** message bodies and recipient addresses MUST NOT be
  logged; error variants must not embed full SMTP conversation secrets;
  `Secret<T>` from platform-core wraps credentials in config types.
- **Ordering:** active change `rust-platform-consumer-baseline` also touches
  arch-tests/CI files; the repo owner selects which lands first and records
  it in `HANDOFF.md` `current_spec` before implementation.

## Capabilities

- `platform-mailing`

## Non-goals

- No HTTP API provider adapters (Resend/Brevo), no router/quota/rate-limit,
  no templates, no DKIM/IMAP, no queue or persistence.
- No login, dashboard, Axum, or SQLx surface (foundation guardrail stands).
- No consumer repository edits and no adoption claims; contract proof from
  three consumers is the extraction evidence, adoption is their own changes.
- No change to `publish = false` or registry strategy.

## Evidence boundary

This change can prove the contract compiles under its feature matrix, the
mock behaves deterministically, error mapping is total, and the dependency
direction oracle still protects the foundation. It cannot prove that any
consumer has migrated or that a real SMTP server accepts the adapter; that
evidence arrives with the per-consumer adoption changes.
