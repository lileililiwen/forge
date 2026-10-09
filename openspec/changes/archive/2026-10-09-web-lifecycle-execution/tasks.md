# Tasks: Web lifecycle execution

## Phase 1 — BFS baseline

- [x] 1.1 Names preflight PASS (`node scripts/check-openspec-change-names.mjs`); `openspec list` shows only `github-gh-fallback-register` + this change; set `current_spec: web-lifecycle-execution` in HANDOFF.
- [x] 1.2 Map touch points: `src/graduation/{validation,import}`, `src/planner/{validate,resolve,apply}`, `src/remediation`, `src/studio`, `src/api/{model,router,command_catalog/*,delivery,maintain}`, `frontend/{index.html,app.js}`, `tests/{portal_ui_contract,lifecycle_rail_browser,browser/lifecycle-rail-check.mjs}`; record current `NotYetWeb` rows and `#wb-idea-entry` string-only state.

## Phase 2 — DFS requirement implementation

- [x] 2.1 Backend `src/api/lifecycle_exec.rs`: graduation preview/import + digest + server-side destination + `graduation.import` journal; intent resolve/apply + digest + `intent.apply` journal; remediate plan/apply + digest + `remediate.apply` journal; delivery next-idea + `delivery.next-idea` journal; wire `Route` variants + router paths + dispatch + permissions + `run_with_operation` replay where applicable.
- [x] 2.2 Catalog: `graduation.preview|import`, `intent.resolve|apply`, `remediate.plan|apply` → `web_exec` with new routes + typed params; keep terminal remediate string; `cargo test --test forge_web_command_catalog_contract` green.
- [x] 2.3 Web idea entry: real form (artifact textarea + profile select + id override, preview → confirm checkbox → run, `role=status` result, error-summary focus, journal reload); studio refine card shows revision bump + journal row.
- [x] 2.4 Web maintain + delivery: catalog-driven intent/remediate cards (plan digest → confirm, stale refused); delivery `#delivery-next-idea` records and shows the journal row; maintain refresh re-renders.
- [x] 2.5 Click oracle: extend `lifecycle-rail-check.mjs` (click every lifecycle button in order, console-error trap, error-summary focus, status updates, per-step screenshots) + `lifecycle_rail_browser.rs` driver (exit 2 UNVERIFIED); fix all true bugs found by clicks.

## Phase 3 — BFS regression/completeness

- [x] 3.1 New `tests/web_lifecycle_execution_contract.rs` green (preview/apply happy paths, missing-confirm refused, stale-digest refused with fresh preview, journal rows, catalog routes, frontend tokens).
- [x] 3.2 `cargo test --test portal_ui_contract --test lifecycle_rail_browser` green; `node --check frontend/app.js`; no new frontend dep; existing terminal strings intact.
- [x] 3.3 CLI parity spot-checks (`graduation preview/import`, `intent resolve/apply`, `remediate plan/apply`, `studio spec/refine`) byte-identical outputs.

## Phase 4 — Verification

- [x] 4.1 `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` PASS; `git diff --check` clean.
- [x] 4.2 `forge gate --dry-run` + bounded `forge gate --timeout-secs 500`; verdict recorded in HANDOFF (attributable failures block; pre-existing recorded with remediation).
- [x] 4.3 Archive without `--skip-specs`; exactly 2 commits on `58d3963` (commit1 impl+specs+UI+playwright, commit2 HANDOFF); never push.
