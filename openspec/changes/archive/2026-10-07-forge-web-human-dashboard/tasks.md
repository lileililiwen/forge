# Tasks: Human-centered dashboard language and visual system

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `frontend/app.js` (all render paths), `frontend/index.html` (sections/copy), `frontend/styles.css` (selector inventory), the surfaced message literals in `src/api/` and the active canonical specs as the reuse contract.
- [x] Map each requirement and scenario to its render function/copy string, display rule and contract/browser-test assertion.
- [x] Extend the browser drive(s) with readable-copy, no-hash-rendered and contrast assertions; extend the frontend source contract with token/order/marker pins.
- [x] Record the exact boundary: display copy + visual system + message literals; wire/codes/statuses/digests/journals untouched.

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/styles.css`: dark command-center token system over all existing selectors; required tokens preserved; every text pair ≥ 4.5:1.
- [x] Fleet rows: name-first, one plain status line, profile words; remove code columns from primary view.
- [x] Cards and action controls: sentence previews/results, no rendered hashes/digests/op-ids (values stay in JS state and wire bodies); pre-fill stage/promote confirmations from status data.
- [x] Onboarding results: auto fleet re-fetch in place, results stay visible, manual Reload retained as fallback.
- [x] Backend message literals: plain-language rewrites with identical codes/statuses; update pinned message assertions.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify every route, code, status, digest, journal shape and catalog row behaves exactly as before (contract suites green).
- [x] Verify heading hierarchy, landmarks, labelled controls, live regions, keyboard flow and responsive breakpoints unchanged in kind.
- [x] Verify the browser drives (readability, contrast, keyboard, no-path) green with stability re-runs.
- [x] Verify no Rust logic, Core, registry, provider or sibling file was modified beyond message literals.
- [x] Remove any current-change placeholder; confirm the visual system covers every shipped panel (fleet, workbench, management, onboarding, portfolio, delivery, commands, login).

## 4. Verification

- [x] `cargo fmt --check` clean; `cargo build` 0 errors.
- [x] `cargo test` green for: extended browser drive(s), `forge_portal_frontend_contract`, `portal_ui_contract`, `forge_web_command_catalog_contract`, `forge_web_command_execution_contract`, `forge_web_project_management_contract`, `forge_web_project_workbench_contract`, `forge_web_workspace_onboarding_contract`, `forge_admin_api_contract`, `forge_web_fleet_contract`; `cargo test --bin forge` and `cargo test --lib api::`. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate` local run with verdict recorded; no new failure attributable to this change.
- [x] Archive with `openspec archive forge-web-human-dashboard --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
