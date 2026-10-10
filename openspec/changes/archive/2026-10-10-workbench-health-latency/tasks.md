# Tasks: Workbench health latency

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/workbench.rs` (`detail`, `build_health`), `src/api/router.rs` (segment table + exhaustiveness arms), `src/api/model.rs` (`Route`), `src/api/admin/deploy.rs` (dispatch + `guarded`), `src/doctor/runner.rs` + `assessment.rs:policy_findings` (None path), `src/policy/driftwatch.rs` (`from_env`, `FORGE_DRIFTWATCH_BIN`), `src/api/command_catalog/` (routes/rows/catalog pins), `frontend/app.js` (`renderHealth`, `HEALTH_LABELS/BADGE`, `requestStatus`), and the active canonical specs as the reuse contract.
- [x] Map each requirement and scenario to its handler, route arm, catalog row, card change and contract-test assertion.
- [x] Add `tests/workbench_health_latency_contract.rs` with the design §8 sentinel scenarios on a throwaway fixture project.
- [x] Record the exact toolchain (Rust 2021, `rustc 1.87` floor) and ownership (extend `src/api` + catalog row + health card; reuse doctor/policy unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/workbench.rs`: split `build_health` into fast local path (None policy + appended `policy-deferred` finding + `deferred` label) and shared `health_document` helper; add `refresh_health` running today's live pass; add `ROUTE_PROJECT_HEALTH_REFRESH` constant.
- [x] `src/api/router.rs` + `model.rs` + exhaustiveness arms + `handlers_core.rs` + `admin/deploy.rs`: thread `Route::AdminProjectHealthRefresh` through, following the maintain arms exactly.
- [x] `src/api/command_catalog/`: reference the new route constant, list it in `IMPLEMENTED_WEB_ROUTES`; no catalog row (direct-called route, same precedent as the maintain GET — `catalog_covers_every_clap_path` count stays 234).
- [x] `frontend/app.js`: `HEALTH_LABELS`/`HEALTH_BADGE` gain `deferred`; `renderHealth` special-cases the `policy-deferred` row with a refresh button + progress state (`refreshHealth`), never a remediate shortcut.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the detail GET shape is unchanged except `state: deferred` + one appended finding when local is clean; `issues`/`stale`/`unavailable` byte-identical in meaning.
- [x] Verify every existing CLI command and flag unchanged; `forge doctor`, `gate`, `check` behavior unchanged.
- [x] Verify no exhaustive `health.state` matcher was missed (compiler + grep over `src/`, `frontend/`).
- [x] Verify old-frontend safety: unknown-state fallback renders, remediate preview on the deferred id refuses honestly.
- [x] Verify no registry/journal schema change; reads still write no rows.
- [x] Remove any current-change placeholder; confirm doctor/policy Core semantics unmodified.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `workbench_health_latency_contract` (new, 5 passed), `forge_web_project_workbench_contract` (10 passed + 1 pre-existing `innerHTML` failure identical on the pristine tree), `forge_web_maintainer_surface_contract` (5 passed), `portal_ui_contract` (21 passed), doctor (39) + policy (42) + api (32) lib tests, `forge_web_command_catalog_contract` (7 passed + 2 pre-existing `cap`-gap failures identical on the pristine tree). Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active (95 passed).
- [x] `git diff --check` clean; review newly added files.
- [ ] Manual: open a project in the dashboard (fast load, Deferred card), run the full check (progress → full card).
- [x] `forge gate --dry-run` rehearse, then full `forge gate --timeout-secs 600`; verdict recorded; no new failure attributable to this change.
- [ ] Archive with `openspec archive workbench-health-latency --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
