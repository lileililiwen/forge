# Tasks: portal-web-ui

## 1. BFS — Baseline and impact coverage
- [ ] 1.1 Map requirements to layers/callers: `src/api/` routes/auth/bounds, `src/portal/` view models, `src/fleet/online.rs` liveness consumer (from `fleet-liveness-status`), Core publish entry, existing API JSON envelopes that must stay byte-identical; record language (Rust, same crate), test layers (unit, HTTP contract without browser engine, live capture), and the implementation-handoff gate.
- [ ] 1.2 Confirm proposal/design/spec agreement on the three routes, confirm-gate/idempotency/auth inheritance, escaping/redaction, and non-goals (no framework, no source editing, CLI parity).

## 2. DFS — Requirement-by-requirement implementation
- [ ] 2.1 Add `src/api/ui.rs` pure HTML renderers (list, detail, plan-preview, operation-accepted, error pages) with escaping unit tests.
- [ ] 2.2 Wire `GET /ui` (roster + publish state + per-row liveness with `unverified` degradation) behind existing API auth.
- [ ] 2.3 Wire `GET /ui/projects/{id}` (evidence detail + portal 404) behind existing API auth.
- [ ] 2.4 Wire `POST /ui/projects/{id}/publish` (plan preview without confirm; 202 + operation id with confirm; idempotency honored; Core preconditions identical to CLI).
- [ ] 2.5 Add `tests/portal_ui_contract.rs` HTTP contract tests (row counts, 404 shape, no-enqueue-without-confirm with journal diff, 202-with-confirm with journal row, escaping, JSON unchanged).

## 3. BFS — Cross-surface regression and completeness
- [ ] 3.1 Prove no existing surface changed: API JSON envelopes, CLI outputs, MCP `tools/list`, portal CLI views byte-identical; no new ports/auth models/dependencies.
- [ ] 3.2 Prove gated-action parity: UI republish and `forge publish` produce equivalent journal rows, operation ids, and refusal strings.

## 4. Verification
- [ ] 4.1 Run `cargo fmt --check` (touched files), `cargo build`, `cargo clippy --all-targets -- -D warnings` (touched files), `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` (full suite green + new suites); `git diff --check` pass.
- [ ] 4.2 Run `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict --no-interactive` (0 failed); re-render ensures proposal/design/specs/tasks still satisfy workflow.md BFS→DFS→BFS, capability, and non-goals sections.
- [ ] 4.3 Live evidence: the three pages captured against the real API + Mac fleet (`api serve` on loopback, browser/CLI capture).
