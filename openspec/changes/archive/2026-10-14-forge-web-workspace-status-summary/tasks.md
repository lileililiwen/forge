# Tasks: forge-web-workspace-status-summary

## 1. BFS baseline

- [x] Read `.ai-rules/workflow.md`, `AGENTS.md`, `.ai-rules/concerns/quality.md` + `security.md`.
- [x] Read `src/cli/setup.rs:27-35` (`cmd_workspace` → `cmd_workspace_sync`) — the CLI workspace sibling overview this mirrors.
- [x] Read `src/api/router.rs` (737 lines; confirmed no new lines may be added; `AdminWorkspaceCandidates` at `:121-123`).
- [x] Read `src/api/workspace.rs:258-325` (`workspace_candidates`) — the existing read-only route and its `{ candidates, total }` response shape.
- [x] Read `frontend/index.html:14-101` — topbar (`<header class="topbar">`) and the project/fleet summary cards.
- [x] Read `frontend/app.js:4061-4258` (`renderWsFleetHint`, `wsDiscover`) and `:6640-6697` (dashboard boot) — the existing candidate loader and its `selectable` predicate.
- [x] Read `tests/forge_web_workspace_onboarding_contract.rs` — confirms the `AdminWorkspaceCandidates` 409/shape boundary is already covered.
- [x] Map requirement → CLI workspace overview → `AdminWorkspaceCandidates` route → topbar summary → failure modes (409 unconfigured, transport error, stale) → test surface.
- [x] Naming preflight: `node scripts/check-openspec-change-names.mjs` → PASS.

## 2. DFS requirement implementation

- [x] Author `openspec/changes/forge-web-workspace-status-summary/proposal.md` (Why, What Changes, BFS Impact Map, Capabilities, Non-goals) citing `forge-web-workspace-onboarding` and `control-plane-portal`.
- [x] Author `openspec/changes/forge-web-workspace-status-summary/design.md` (ownership, contracts, failures, migrations, compatibility).
- [x] Author `openspec/changes/forge-web-workspace-status-summary/tasks.md` (this file).
- [x] Author the spec delta `openspec/changes/forge-web-workspace-status-summary/specs/control-plane-portal/spec.md` (new topbar workspace summary requirement + scenarios, including the honest 409 empty state).

## 3. BFS regression/completeness

- [x] Single-change validate: `openspec validate forge-web-workspace-status-summary --strict --no-interactive` → PASS.
- [x] Commit 1: `openspec/changes/forge-web-workspace-status-summary/` only (`proposal: forge-web-workspace-status-summary`).
- [x] Implementation: `frontend/index.html` — additive topbar workspace-summary chip group (`#ws-summary` with `#ws-registered`, `#ws-onboardable`, `#ws-last-sync`) plus a `#ws-summary-empty` "not configured" chip; placed inside the topbar left cluster, hidden until loaded.
- [x] Implementation: `frontend/app.js` — `aggregateWorkspaceSummary(candidates)` (pure helper) + `loadWorkspaceSummary()` wired from the dashboard boot (`frontend/app.js:6677` beside `wsDiscover()`); `textContent`-only, `role="status"` + `aria-live="polite"`, no `innerHTML` / `eval(` / `Function(`, no write button; 409 → show `#ws-summary-empty`, hide counters; last-sync relative + stale label.
- [x] Implementation: `frontend/styles.css` — minimal `.ws-summary` / `.ws-summary-chip` rules reusing existing tokens; otherwise reuse `.badge-unconfigured` / `.muted` / `.topbar`.
- [x] Test: `tests/web_workspace_status_summary_contract.rs` — static-token contract over shipped `frontend/` (summary region present in `index.html`, loader wired in `app.js`, no banned primitives, no write button) + an aggregation assertion over a candidate-state fixture; **skip 401/403/415** (already covered by `tests/forge_web_workspace_onboarding_contract.rs`); design notes the boundary is pre-existing.
- [x] Commit 2: implementation + tests (`impl: forge-web-workspace-status-summary (topbar counters)`).
- [x] Pre-existing tests still passing per single-change validate: **yes** (`openspec validate --strict --no-interactive` is the only validate run here; the manager runs `--all --strict` and `cargo test --workspace`).

## 4. Verification

- [x] **Lightweight only** (manager runs the heavy pass at the end):
  - [x] `node scripts/check-openspec-change-names.mjs` → PASS.
  - [x] `wc -l src/api/router.rs` ≤ 1000 (cap respected — zero lines added to `router.rs`).
  - [x] `git diff --check` → clean.
  - [x] `openspec validate forge-web-workspace-status-summary --strict --no-interactive` → PASS.
  - [x] Source-file-size cap respected on every new/touched file.
  - [x] No `cargo build`, no `cargo test --workspace`, no `forge gate`, no `openspec validate --all --strict` — manager-owned.
- [x] **Manager-owned** (not run here):
  - [ ] `cargo build`, `cargo test --workspace` (full pre-existing test suite — must stay green; `router.rs` untouched so the table-driven dispatch stays exhaustive).
  - [ ] `openspec validate --all --strict --no-interactive` (full active + archive + canonical spec set).
  - [ ] `openspec archive forge-web-workspace-status-summary --yes` (no `--skip-specs`; promotes the `control-plane-portal` delta; archived as `2026-10-14-forge-web-workspace-status-summary`).
  - [ ] HANDOFF update: record the deliverable + evidence table; advance the `current_spec` pointer to the next active eligible change (or remove it if none remain).
  - [ ] Commit HANDOFF; stop (no push, no further changes).
