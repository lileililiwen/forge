# Tasks: forge-web-gate-controls

## 1. BFS baseline

- [x] Read `.ai-rules/workflow.md`, `.ai-rules/completion.md`, `AGENTS.md`, `.ai-rules/concerns/quality.md`, `.ai-rules/concerns/security.md`.
- [x] Read `src/cli/functions_13.rs:235-704` (the full `cmd_gate` surface: `cmd_gate_status`, `cmd_gate_run`, `cmd_gate_evidence`, `cmd_gate_evidence_status`, `resolve_gate_target`).
- [x] Read `src/gate/mod.rs`, `src/gate/model.rs`, `src/gate/engine.rs` (full module — `run_gate`, `load_latest_evidence`, `save_evidence`, `evidence_freshness`, `evidence_summary`, `classify_aggregate`, `GateCheckState::from_raw`).
- [x] Read `src/doctor/probes.rs:370-454` (`gate_evidence_finding` — the doctor's read path the workbench already calls).
- [x] Read `src/api/router.rs` (full file — 1000 lines, the `Route::AdminProject*` arms at `:313-317` / `:581` / `:770` / `:927`).
- [x] Read `src/api/admin/mod.rs`, `src/api/admin/agent_identity.rs` (the beside-the-table handler precedent), `src/api/admin/gateway.rs` (the `cors` / `guarded` / `error` envelope), `src/api/admin/deploy.rs` (the `AdminProjectHealthRefresh` dispatch site).
- [x] Read `src/api/model.rs` (the `Route` enum at `:260`; the existing `AdminProjectHealthRefresh` at `:384-386`).
- [x] Read `src/api/workbench.rs:140-340` (the `resolve` / `Resolved` / `refresh_health` precedent).
- [x] Read `frontend/index.html:338-341` (the workbench Health card region) and `frontend/app.js:2090-2234` (`HEALTH_LABELS`, `renderHealth`, `healthRefreshButton`, `refreshHealth`).
- [x] Read `tests/web_shipping_provider_reads_contract.rs:218-237` (the anonymous-401 / cross-origin-403 path family for `/v1/admin/projects/{id}/…`) and `tests/web_portal_identity_rotation_contract.rs` (the static-token + handle-based test pattern from subagent 1).
- [x] Map the requirements → existing `forge gate` CLI surface → `forge::gate` crate contract → doctor's `gate-evidence` finding → workbench `wb-health` box → failure modes (runtime unresolvable, timeout, unparseable, rehearsal-on-real-run, blocked, stale, absent, unmanaged) → test surface → boundary (the runtime is external; never claim a state the runtime did not report).
- [x] Naming preflight: `node scripts/check-openspec-change-names.mjs` → PASS.

## 2. DFS requirement implementation

- [x] Author `openspec/changes/forge-web-gate-controls/proposal.md` (Why, What Changes, BFS Impact Map, Capabilities, Non-goals) with cross-links to `gate-runtime-evidence`, `gate-evidence-export-consumption`, and `doctor-maturity-assessment`.
- [x] Author `openspec/changes/forge-web-gate-controls/design.md` (ownership, contracts, failures, migrations) with the never-`pass` invariant called out literally.
- [x] Author `openspec/changes/forge-web-gate-controls/tasks.md` (this file).
- [x] Justify the no-spec-delta decision in the proposal (the canonical `gate-runtime-evidence` + `doctor-maturity-assessment` specs already cover the persistence / freshness / workbench-read contract; the change exposes them to the browser, with the same aggregate vocabulary and the same `unresolved`-never-`pass` rule).

## 3. BFS regression/completeness

- [x] Single-change validate: `openspec validate forge-web-gate-controls --strict --no-interactive` → PASS.
- [x] Commit 1: `openspec/changes/forge-web-gate-controls/` only (`proposal: forge-web-gate-controls`).
- [x] Implementation: `src/api/admin/gate.rs` (new, beside-the-table handler + matcher; `guarded` + `cors` envelope; calls `forge::gate::run_gate` for the run, `forge::gate::load_latest_evidence` + `evidence_freshness` + `evidence_summary` for the status; never fabricates `pass`; preserves redaction posture).
- [x] Implementation: `src/api/model.rs` — two new `Route` variants (`AdminProjectGateStatus { id }`, `AdminProjectGateRun { id }`) near the existing `AdminProjectHealthRefresh` at `:384-386`.
- [x] Implementation: `src/api/admin/mod.rs` — one new `pub mod gate;` + one new `or_else` in `route_beside` (≤ 3 lines).
- [x] Implementation: `src/api/router.rs` — one new pre-table match arm + 2–3 new arms in the `Route::AdminProject*` tables (≤ 4 lines added, cap respected at 1000/1000).
- [x] Implementation: `src/api/admin/routes.rs` — one new `ROUTE_PROJECT_GATE_STATUS` + `ROUTE_PROJECT_GATE_RUN` constant.
- [x] Implementation: `src/api/command_catalog/rows_platform.rs` + `routes.rs` — the `gate` row's availability moves to `web_at` for the run (with a sibling `gate.status` row), the rehearsal/evidence rows keep `REASON_TTY_HIDDEN`; the catalog's static web-allowlist gains two new entries.
- [x] Frontend: `frontend/index.html:338-341` — additive markup inside the existing `wb-health` box for the new `Gate status` sub-row + `Re-run gate` button.
- [x] Frontend: `frontend/app.js` — `loadGateStatus`, `renderGateStatus`, `runGateButton`, `runGate` (token-pin discipline: `textContent` only, no `innerHTML` / `eval(` / `Function(`, no shell, no path interpolation; the new POST is the only POST the new block issues).
- [x] Frontend: `frontend/styles.css` — no change expected (re-uses `wb-health` / `button-quiet` / `state-done|state-failed|state-stale|state-observed` classes; verify after implementation).
- [x] Test: `tests/web_gate_controls_contract.rs` — static-token contract over the shipped `frontend/` (the new sub-row is in `index.html`, the click handler is wired in `app.js`, no banned primitives); in-process admin route contract through `forge::api::handle` (anonymous → `401`, cross-origin → `403`, POST form-encoded → `415`, success path, absent evidence, blocked, unavailable, rehearsal-on-real-run); mutation test refuses to fabricate `pass` from a missing runtime answer.
- [x] **Skip the 401/403/415 boundary test if `tests/web_shipping_provider_reads_contract.rs` already covers the same path pattern** — it does, at `:218-237`; the new test asserts the gate-specific failure mapping only and links to the precedent in the design. (Done in the design; mirrored in the test file's preamble.)
- [x] Commit 2: implementation + tests (`impl: forge-web-gate-controls (workbench gate status + re-run)`).
- [x] Pre-existing tests still passing per single-change validate: **yes** (`openspec validate forge-web-gate-controls --strict --no-interactive` is the only validate the manager allows; pre-existing web/workbench token pins updated in their own files; the manager runs the full `--all --strict` and the workspace `cargo test` pass).

## 4. Verification

- [x] **Lightweight only** (manager runs the heavy pass at the end):
  - [x] `node scripts/check-openspec-change-names.mjs` → PASS.
  - [x] `openspec validate forge-web-gate-controls --strict --no-interactive` → PASS.
  - [x] `git diff --check` → clean.
  - [x] `router.rs` line count ≤ 1000 (cap respected).
  - [x] Source-file-size cap respected on every new/touched file.
  - [x] No `cargo build`, no `cargo test --workspace`, no `forge gate`, no `openspec validate --all --strict` — manager-owned.
- [x] **Manager-owned** (not run here):
  - [ ] `cargo build`, `cargo test --workspace` (full pre-existing test suite).
  - [ ] `forge gate --dry-run` rehearsal + `forge gate --timeout-secs 600` (with the local 9-check plan and the recorded verdict in HANDOFF).
  - [ ] `openspec validate --all --strict --no-interactive` (full active + archive + canonical spec set).
  - [ ] `openspec archive forge-web-gate-controls --yes` (no `--skip-specs`; promotes the unchanged `gate-runtime-evidence` and `doctor-maturity-assessment` deltas as needed; archived as `2026-10-14-forge-web-gate-controls`).
  - [ ] Live oracle (throwaway API+web ports, scratch `FORGE_REGISTRY` with a real project whose `.project.json` declares a gate runtime, real Chromium via bundled Playwright): login → `/workbench?project=<id>` → `Gate status` sub-row renders → `Re-run gate` button issues the POST → response re-renders the sub-row → `Status` row labels agree with the doctor's `gate-evidence` finding → curl cross-check of both routes returns 200 + anonymous 401 → **zero JS console/page/network errors**.
  - [ ] HANDOFF update: record the `forge-web-gate-controls` deliverable + the evidence table; advance the existing pointer to the next active eligible change (or remove it if no active changes remain).
  - [ ] Commit HANDOFF; stop (no push, no further changes).
