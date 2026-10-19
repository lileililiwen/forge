# Tasks: platform-mailing crate and satellite dependency policy

## 0. Caveats and confirmations

- [ ] **Do not edit sibling repositories.** Consumer migration (alltools,
  openpanel, rust-ecommerce) happens only in each consumer's own OpenSpec
  change. This package must not touch them.
- [ ] **Foundation neutrality is restated, not weakened.** The four
  foundation crates must remain free of lettre/tokio; only the named
  satellite `platform-mailing` gains the allowlist entry.
- [ ] **`ErrorCategory` enum stays frozen** (design L3). If the mapping in
  design §5 seems impossible against the shipped variants, stop and record
  the exact conflict; do not extend platform-core inside this change.
- [ ] **Ordering with active changes.** `rust-platform-consumer-baseline` and
  `post-mvp-readiness` are open; both may touch arch-tests/CI. Before any
  implementation, the repo owner selects the sequence and sets the single
  `current_spec` line in `HANDOFF.md`.
- [ ] **Record environment-blocked checks** (e.g. absent cargo-deny) as
  BLOCKED with the exact command; never as passing.
- [ ] **Keep `publish = false`.** Registry publication is roadmap decision D4.

## 1. BFS — Baseline and impact coverage

- [ ] Read `openspec/specs/rust-platform-foundation/spec.md`,
  `crates/platform-core/src/lib.rs` (ErrorCategory, SafeError, Secret,
  Clock, CorrelationId), and `crates/platform-arch-tests/src/` rule layout.
- [ ] Map each requirement MAIL-001..004 and every design §6 failure row to
  its test location from design §7; confirm the traceability table in
  design.md matches proposal scope.
- [ ] Confirm the lettre 0.11 feature set (`smtp-transport`, `tokio1`,
  `rustls-tls`) resolves against the workspace lockfile policy with no
  native-tls edge (`cargo add --dry-run` equivalent check).
- [ ] Verify the implementation-handoff gate: language, toolchain (1.75,
  edition 2021), crate boundary (satellite), and shared owner
  (rust-platform-libs) are as recorded in design §1–§3.
- [ ] Add test skeletons for validation and error-mapping cases (failing
  compile of the public surface only; no behavior claimed).

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 MAIL-001: `types.rs` — `EmailAddress` parser with §4 limits and
  boundary tests (254-byte, control chars, CR/LF subject, 5 MiB body, 51st
  recipient, empty `to`); `EmailMessage`, `Body`, `AttachmentRef` builders.
- [ ] 2.2 MAIL-001: `error.rs` — `MailingError` variants, total mapping to
  existing `ErrorCategory`, redaction tests asserting no credential/body/
  local-part leakage in `Display`.
- [ ] 2.3 MAIL-001: `transport.rs` — `MailTransport` (async-trait),
  `SendContext`, `DeliveryReceipt` with injected-clock `SentAt`.
- [ ] 2.4 MAIL-002: `smtp.rs` under feature `smtp-lettre` —
  `SmtpTransportConfig` (loopback-only rule for `Security::None`),
  `LettreSmtpTransport`, lettre error → `MailingError` mapping covering
  every `lettre::transport::smtp::ErrorKind` (§7 total-mapping test).
- [ ] 2.5 MAIL-003: `mock.rs` under feature `mock` — scripted
  success/failure/timeout outcomes, no network, deterministic timestamps.
- [ ] 2.6 MAIL-004: `platform-arch-tests` — refactor the rule into a pure
  function over (crate, graph) with a named-satellite allowlist table;
  foundation crates unchanged; add the lettre-on-platform-identity
  synthesized-graph negative unit test; register `platform-mailing` in the
  workspace `Cargo.toml`.

## 3. BFS — Cross-surface regression and completeness

- [ ] Run the full feature matrix (design §2 commands) and confirm every
  §6 failure row has a passing test or a recorded BLOCKED reason.
- [ ] `cargo tree -p platform-mailing --no-default-features` proves no
  lettre/tokio edge; `--features smtp-lettre,mock` proves it builds; default
  workspace test run unaffected.
- [ ] Re-inspect `platform-core|contracts|identity|audit|testing` public
  APIs and their tests for zero drift; confirm `README.md` moves mailing
  from "Deferred" to "Implemented", `ROADMAP.md` records the extraction
  decision per its own deferral wording, and no placeholder/TBD text
  remains from this change.
- [ ] Confirm no sibling repository was modified.

## 4. Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`
- [ ] `cargo test -p platform-mailing --no-default-features` and
  `cargo test -p platform-mailing --features smtp-lettre,mock`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `node scripts/check-openspec-change-names.mjs`
- [ ] `openspec validate --changes --strict --no-interactive`
- [ ] Record evidence in `HANDOFF.md`; completion follows the repo's
  two-commit cadence (implementation+archive, then HANDOFF pointer).
