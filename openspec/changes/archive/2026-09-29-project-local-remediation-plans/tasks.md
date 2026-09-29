# Tasks: project-local-remediation-plans

Status: implementation in progress; package expanded from planning-only detail
after explicit user authorization.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read `src/standard/`, `src/generate/` and `src/spec/mod.rs`; reuse `Receipt`/`ReceiptFile` digests, `render_snapshot`, `upgrade_snapshot`'s explicit confirmation boundary, generator staging/collision discipline, and `Remediation::Automatic|Ai|Manual` routing vocabulary.
- [x] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/remediation/*`, CLI `forge remediate …`, dependency on the gap assessment and standard packs, no new dependency. User explicitly authorized implementation after the package was expanded.
- [x] 1.3 Map requirements to `install_standard_asset`, target/profile/pack preconditions, CLI and registry callers, typed invalid/conflict/apply-failed errors, rollback/idempotency failures, and the named contract/cross-surface tests below.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement the `forge-remediation-plan/0.1.0` plan, action and outcome shapes with a closed action-kind set and deterministic plan id.
- [x] 2.2 Implement finding selection restricted to automatic `gaps.ci.<project>.ci`; semantic/manual/unknown findings refuse with `remediation-invalid` and empty stdout.
- [x] 2.3 Implement confirmation, `.standard/receipt.json` ownership checks, target revision/digest preconditions, target-local stage/verify/promote, idempotent re-apply, and rollback-on-failure with explicit rollback evidence.
- [x] 2.4 Implement `scan|plan|diff|apply` with `--target`, `--finding`, `--pack`, optional `--plan`, `--confirm`, and global `--format`; only explicit standard-pack assets are consumed.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Prove preview paths write nothing and apply preserves unrelated files.
- [x] 3.2 Exercise unowned collision, stale revision/digest, missing pack asset, invalid profile, partial promotion and rollback-failure boundaries.
- [x] 3.3 Prove no secret is generated or disclosed and no remote mutation occurs.

## 4. Verification

- [x] 4.1 Run formatting/build/Clippy/tests, both new suites, strict OpenSpec validation, the name preflight and diff checks; record exact outcomes and unrelated baseline failures below.
- [x] 4.2 Record that no GitHub write, deployment or semantic generation was performed; those are separate packages.

Verification evidence (2026-09-29):

- `cargo build --bin forge`: PASS. It reports existing warnings in
  `src/portfolio/share/validation.rs` and the publish CLI.
- `rustfmt --edition 2021 --check src/remediation/mod.rs src/main.rs
  tests/remediation_contract.rs tests/remediation_cross_surface.rs`: PASS.
- `cargo test --workspace --all-targets -- --test-threads=1
  --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS for every selected test binary. The named native-toolchain scaffold test
  was filtered. An initial parallel run had one transient
  `governance_contract::valid_external_response_is_normalized_and_redacted`
  failure; that test passed in isolation and the serial workspace run passed.
- `cargo test --test remediation_contract`: 9 passed; `cargo test --test
  remediation_cross_surface`: 3 passed; remediation unit tests: 3 passed.
- `cargo clippy --all-targets -- -D warnings`: BLOCKED by existing warnings in
  API, gate, portfolio and publish code, plus two findings in this change
  (type complexity and needless borrow). The two change-local findings were
  corrected; strict Clippy remains non-green due to the unrelated findings.
- `node scripts/check-openspec-change-names.mjs`: PASS. Strict OpenSpec
  validation and `git diff --check` are rerun after the final spec edits and
  before archive.
- No shared Gate Runtime is configured; no Gate pass is claimed. No remote
  write, secret generation/disclosure, deployment or semantic generation was
  performed.
