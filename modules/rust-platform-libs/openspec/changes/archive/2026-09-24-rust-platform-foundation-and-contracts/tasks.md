# Tasks: Rust platform foundation

## 1. BFS — Baseline and impact coverage

- [x] Inventory repeated Rust identity, session, admin, error, audit, and test
  patterns in openaccounting, rust-ecommerce, hookit, openpanel, and vowra.
  The proposal explicitly defers consumer migration to a follow-up change; the
  foundation exposes only traits and types, so the BFS is the list of repeated
  shape decisions (subject/session/permission/tenant/lifecycle). Recorded as
  the crate boundaries in `crates/platform-identity/src/traits.rs` and
  `crates/platform-identity/src/{subject,session,permission,tenant,lifecycle}.rs`.
- [x] Map each proposed crate API to the language-neutral contract schemas and
  identify fields that must remain application-owned. Mapping is in
  `crates/platform-contracts/src/payload/*.rs` (one module per contract
  family). Application-owned fields (user records, password hashes, session
  storage, provider config, migrations) are explicitly listed in
  `crates/platform-identity/src/lib.rs` as out-of-scope.
- [x] Define dependency-direction and forbidden-dependency tests before adding
  crates. Defined in `crates/platform-arch-tests/src/lib.rs` and
  `crates/platform-arch-tests/tests/dependency_direction.rs`; the tests run
  before any production crate is added to the lockfile.

## 2. DFS — Requirement-by-requirement implementation

- [x] Create the Cargo workspace and `platform-core`. `Cargo.toml` at the
  workspace root; `crates/platform-core` provides bounded strings, opaque
  `Id<T>`, `Clock`/`SystemClock`/`FixedClock`, `CorrelationId`, `Secret<T>`,
  `ErrorCategory`/`SafeError`, and `PlatformResult`.
- [x] Add `platform-contracts` representations and fixture compatibility tests.
  `crates/platform-contracts` ships the envelope, registry loader, JSON
  Schema-driven validator, and the per-family payload types. Compatibility
  tests live in `crates/platform-contracts/tests/fixture_parity.rs` and walk
  the live `platform-contracts` fixture matrix.
- [x] Add provider-neutral identity, permission, and tenant contracts.
  `crates/platform-identity` exposes `Subject`, `Session`, `Permission`,
  `Tenant`, `Lifecycle`, `IdentityError`, `IdentityOutcome`, and the
  `IdentityStore`/`IdentityProvider`/`CurrentSubject` traits. No
  persistence, no provider config, no HTTP — applications own all of those.
- [x] Add audit contracts and safe failure categories.
  `crates/platform-audit` exposes `AuditEvent`, `AuditEventBuilder`,
  `RedactedContext` (rejects secret-named keys at construction), and the
  `AuditSink` trait. `NoopAuditSink` and `InMemoryAuditSink` are shipped
  for tests and the cross-repo parity walker.
- [x] Add test-only deterministic support with production dependency guards.
  `crates/platform-testing` is the only crate that depends on every other
  platform crate. The `platform-arch-tests` crate enforces that no
  production crate references `platform-testing` directly or transitively.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run the dependency matrix and ensure no foundation crate imports Axum,
  SQLx, cookies, OIDC, JWT, or application code. Covered by
  `cargo test -p platform-arch-tests` (4 integration tests + 3 unit tests);
  the lockfile walk inspects transitive deps for the 17-entry forbidden list.
- [x] Exercise unknown fields, unsupported versions, redaction, empty values,
  and safe error output. Covered by
  `crates/platform-contracts/tests/fixture_parity.rs` (48 fixtures across 4
  contract families, each with the 12 canonical cases) plus
  `crates/platform-identity/tests/safety.rs` and
  `crates/platform-audit/tests/safety.rs`.
- [x] Confirm no existing business project is migrated by this foundation
  change; adoption remains a later consumer-specific change. Verified by
  `cargo test -p platform-arch-tests --test dependency_direction
  production_crates_listed`: the only `dependencies` graph in this
  workspace is the foundation graph itself. The proposal's non-goals
  forbid migrating `openaccounting`, `rust-ecommerce`, `hookit`,
  `openpanel`, or `vowra` in this change.

## 4. Verification

- [x] `node scripts/check-openspec-change-names.mjs` → ok.
- [x] `cargo fmt --all -- --check` → clean.
- [x] `cargo test --workspace` → 69 passed, 0 failed (in this commit).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- [x] `openspec validate --changes --strict --no-interactive` → 1/1 passed.
- [x] `openspec validate --specs --strict --no-interactive` → 1/1 passed
  (`spec/rust-platform-foundation`) after `openspec archive` synced the
  spec delta.
- [x] Cross-repository fixture check: BLOCKED on full parity sweep. The
  foundation walks the live `platform-contracts` fixture matrix (48
  cases × 4 families) and matches every outcome, but a 9-family sweep
  (readiness, gate-result, job-outcome, release-evidence, capability)
  is the next consumer-conformance-matrix change, not this foundation.
  See the `Evidence boundary` block in `HANDOFF.md` for the exact
  next action and the requirement that the foundation does not
  claim consumer compatibility from fixture parity alone.
