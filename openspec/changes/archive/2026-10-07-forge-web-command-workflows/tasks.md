# Tasks: Task-first dashboard with one-confirm bulk onboarding

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `frontend/index.html` (section/nav order, heading hierarchy), `frontend/app.js` (dashboard load order, `wsDiscover`/`wsPreview`/`wsRun`, request helpers), the shipped `candidates`/`onboard` contract and the active spec as the reuse contract.
- [x] Map each requirement and scenario to its DOM change, flow change and browser/source-test assertion.
- [x] Extend `tests/browser/workspace-onboarding-check.mjs` + `tests/forge_web_workspace_onboarding_browser.rs` to a 27-dir fixture with order, auto-discovery, chunked-confirm and fleet-appears assertions.
- [x] Record the exact boundary: `frontend/` only; no Rust, catalog, Core or journal change.

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/index.html`: move `#commands` after `#delivery`, reorder sidebar nav to fleet → workbench → management → portfolio → delivery → commands, relabel commands copy as reference; add the unonboarded-count element under the fleet heading.
- [x] `frontend/app.js`: auto-run discovery on dashboard load; render the unonboarded-count line (hidden when unconfigured); chunk preview/apply at 25 with one combined plan, per-chunk digests and sequential apply that stops honestly on refusal.
- [x] Update the Playwright drive and fixture for auto-discovery, 27-dir two-chunk confirm, section order, keyboard, contrast and no-path assertions.
- [x] Update the frontend source contract for section order, count element, auto-discover and chunking markers.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify heading hierarchy, skip link, labelled controls, live regions and table keyboard regions are unchanged in kind; single-item controls, delivery, portfolio and catalog rendering untouched.
- [x] Verify discovery failure and unconfigured root degrade to the manual path with honest notices.
- [x] Verify a refused second chunk keeps first-chunk results and prompts re-preview without inventing success.
- [x] Verify no API/catalog/Core file was modified beyond the intended frontend/test surfaces.
- [x] Verify the Chromium flow against a throwaway 27-dir root.

## 4. Verification

- [x] `cargo fmt --check` clean (JS untouched by fmt; eyeball style consistency).
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_workspace_onboarding_browser` (extended), `forge_portal_frontend_contract`, `portal_ui_contract`, `forge_web_command_catalog_contract`, `forge_web_project_management_contract`, `forge_web_project_workbench_contract`; `cargo test --bin forge` and `cargo test --lib api::`. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate` local run with verdict recorded; no new failure attributable to this change (frontend-only: none possible beyond the existing baseline).
- [x] Archive with `openspec archive forge-web-command-workflows --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
