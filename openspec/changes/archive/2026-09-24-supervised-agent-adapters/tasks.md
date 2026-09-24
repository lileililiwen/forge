# Tasks: Wire Forge agent sessions to the real supervised runtimes

## 1. BFS — Baseline and impact coverage

- [x] Re-read the sibling docs (`ariadex` lifecycle verbs, `sisyphusfy
  --json` outcome shape) and record exact command/output contracts in the
  fixture notes.
- [x] Map `src/agent` state machine, transition routing, run-spec path and
  every caller (CLI, MCP `run_agent`).
- [x] Define `RuntimeBacking` serde default and session-file compatibility
  test skeleton.
- [x] Add stub-binary fixtures: ariadex start/status/resume/pause/attach
  guidance, unknown session; sisyphusfy done/retryable/malformed JSON.

## 2. DFS — Requirement-by-requirement implementation

- [x] Implement ordered binary resolution + bounded invocation helpers
  shared by both providers.
- [x] Implement the ariadex provider adapter mapping every transition to
  the sibling's real verb or an operator-path refusal.
- [x] Implement the sisyphusfy run-spec adapter and evidence ingestion
  (verification verdict only).
- [x] Persist `backing` on session records; surface runtime identity in
  `agent status|inspect` human and JSON output.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-verify bundled OpenCode/Codex adapter behavior unchanged
  (`backing: None` path).
- [x] Re-verify session preservation when the runtime disappears mid-life
  and the spec-missing boundary still holds.
- [x] Re-verify MCP `run_agent` parity and portal read surfaces; journal
  kinds intact.
- [x] Confirm no shell interpolation, no vendored sibling logic, no
  credential capture outside redaction.

## 4. Verification

- [x] `cargo fmt --all -- --check`; `cargo build`; `cargo test
  --all-targets`; `cargo clippy --all-targets -- -D warnings`.
- [x] `node scripts/check-openspec-change-names.mjs`; `openspec validate
  --all --strict --no-interactive`; `git diff --check`.
- [x] Live round trips with real ariadex/sisyphusfy where installed;
  record evidence or exact blocker per provider.
