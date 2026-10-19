# Tasks: Identity, audit, and permission contracts

## 1. BFS — Baseline and impact coverage

- [x] Identify the audit emission patterns consumers need:
  authentication success/failure, permission evaluation, session
  lifecycle. Cataloged in `design.md` as the `AuthenticationAuditor`,
  `PermissionAuditor`, and `SessionAuditor` method matrix.
- [x] Identify the permission evaluation patterns: deny-by-default
  (the safe default), allow-list (the smallest useful non-trivial
  policy). Cataloged in `design.md` as `DenyByDefaultEvaluator` and
  `AllowListEvaluator`. No role inheritance, no policy DSL.
- [x] Identify the tenant resolution patterns: from a resolved
  `Subject`. Cataloged in `design.md` as the `TenantResolver` trait
  with a `clock` parameter for the `active_at` timestamp and a
  `correlation_id` parameter for log aggregation.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add `PermissionEvaluator` trait + `PermissionEvaluation`
  result type in `crates/platform-identity/src/evaluator.rs`.
- [x] Add `DenyByDefaultEvaluator` and `AllowListEvaluator` reference
  implementations in the same file, with unit tests for every
  branch.
- [x] Add `TenantResolver` trait + `NoTenantResolver` reference
  implementation in `crates/platform-identity/src/tenant_resolver.rs`.
- [x] Add `AuthenticationAuditor`, `PermissionAuditor`,
  `SessionAuditor` in `crates/platform-audit/src/producers.rs`.
  Each producer re-checks the `RedactedContext` policy at
  construction time and never embeds a free-form error message.
- [x] Add `ScriptedPermissionEvaluator` in
  `crates/platform-testing/src/evaluator.rs`. Falls back to a
  configurable default (default: `DenyByDefaultEvaluator`) once
  the script is exhausted.
- [x] Add an end-to-end identity -> permission -> audit
  integration test under `crates/platform-testing/tests/`
  that exercises the new surface in a single test and asserts
  the audit events land in the in-memory sink with the expected
  category, outcome, and redaction. (Lives in `platform-testing`
  because that crate already has access to all three surfaces;
  `platform-identity` does not depend on `platform-audit` by
  design.)
- [x] Re-run the foundation's 48-case cross-repository fixture
  parity walker to prove the new code did not regress the
  `platform-contracts` validator.

## 3. BFS — Cross-surface regression and completeness

- [x] All existing foundation tests still pass.
- [x] The foundation's language-neutral contract parity walker
  still passes for all 4 contract families.
- [x] The architecture test's forbidden-dep list still rejects
  `axum`, `sqlx`, `cookie`, `jsonwebtoken`, `openidconnect`,
  `oauth2`, `reqwest`, `tokio`, and `platform-testing` for every
  production crate, directly or transitively.
- [x] The new surface is documented in `crates/platform-identity/src/lib.rs`
  and `crates/platform-audit/src/lib.rs` so a future consumer can
  find the new types without re-reading the spec.

## 4. Verification

- [x] `node scripts/check-openspec-change-names.mjs` -> ok.
- [x] `cargo fmt --all -- --check` -> clean.
- [x] `cargo test --workspace` -> existing 69 foundation tests
  plus the new surface tests (21 new), all 90 passing.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` ->
  clean.
- [x] `openspec validate --changes --strict --no-interactive` ->
  1/1 passed.
- [x] `openspec validate --specs --strict --no-interactive` (after
  archive) -> 1/1 passed for `spec/rust-identity-audit-permission`
  on top of the existing `spec/rust-platform-foundation`.
- [x] HANDOFF.md updated to point to the new evidence commit
  and to keep the same evidence boundary language (4-of-9
  contract families, validator agreement not semantic consumer
  fidelity, foundation is provider-neutral but applications
  are not).
