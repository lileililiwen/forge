# Tasks: web-release-deploy-history

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/api/admin/deploy.rs` (dispatch + `deploy_id_gate` + `guarded`/`cors`/`is_json`), `src/api/admin/release.rs` (`release_gate`, `typed_release_error`, scrub precedent), `src/api/router.rs` (deploy/release arms + OPTIONS + permission/exhaustiveness lists), `src/api/model.rs` (`Route::AdminProjectDeploy*` docs), `src/api/command_catalog/routes.rs` + `rows_shipping.rs` (five `NotYetWeb` leaves, `IMPLEMENTED_WEB_ROUTES`, 234 count pin), `src/release/engine/operations.rs` (`list_releases`/`read_release`), `src/deploy/engine/stages.rs` (`list_deploys`/`read_deploy`), `src/registry/model.rs` (`operations_for_project`), `src/cli/release.rs` (`filter_publish_deploy`, status envelope), `frontend/index.html` (Delivery section) + `frontend/app.js` (delivery region + catalog-browser read pattern), and canonical `release-publishing` + `adapter-deployment` + `portal-web-ui` specs.
- [x] Map each requirement/scenario to handler, route arm, catalog row, card control and contract-test assertion; record toolchain and ownership (extend `src/api` + catalog + Delivery card; Core/CLI/schema unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/admin/routes.rs` + `src/api/admin/history.rs` (new): five route constants + five session-gated GET handlers reusing `deploy_id_gate`, `guarded`, `cors`, `error`, `scrub_*`; inspect-id + limit validation; path-free projections (`state_path` omitted); journal-filtered status; every response carries `contract: API_CONTRACT_VERSION`. No write, no provider, no adapter, no shell.
- [x] `src/api/model.rs` + `router.rs` + `admin/deploy.rs`: five `Route` variants, GET arms, OPTIONS coverage, permission + dispatch + exhaustiveness threading; no existing arm shadowed.
- [x] `src/api/command_catalog/`: five leaves `NotYetWeb` → `web_at` + `IMPLEMENTED_WEB_ROUTES` entries; count pin stays 234; `deploy.observe` untouched.
- [x] `frontend/index.html` + `frontend/app.js`: Delivery "Release & deploy history" card reusing `#delivery-project` — list buttons, inspect inputs, status read; `role=status` results, error-summary focus, native controls, no new dependency, no `innerHTML`, no inline script.
- [x] `tests/web_release_deploy_history_contract.rs`: design §8 oracle (auth, gates, lists, inspects, status filter, limit, scrub, catalog + frontend pins).
- [x] Delta spec scenarios implemented alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Existing deploy/release plan/apply shapes byte-identical; refused history reads create no state dirs or journal rows and probe no provider.
- [x] CLI `release list|inspect`, `deploy list|inspect|status` behavior unchanged; no registry/journal schema change.
- [x] Full forbidden-sink + inline-script + path-leak sweep over the new surface (Rust + frontend).
- [x] No current-change placeholder/disabled behavior remains; `IMPLEMENTED_WEB_ROUTES` agrees with all pins; old-frontend safety (unknown ids still typed refusals).
- [x] New/touched Rust files stay under the source-file-size cap.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (only pre-existing warnings).
- [x] `cargo test --test web_release_deploy_history_contract` green; regression: lib, `forge_web_project_release_contract`, `forge_web_project_deployment_contract`, `forge_web_project_delivery_contract`, `forge_web_command_catalog_contract`, `portal_ui_contract`. No new failure (pre-existing failures recorded with pristine-tree evidence + remediation).
- [x] `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` 0 failures; `git diff --check` clean.
- [x] Live throwaway-registry browser/curl proof: login → Delivery history exercises lists/inspects/status with zero attributable JS errors.
- [x] `forge gate --dry-run` rehearse, then full bounded `forge gate --timeout-secs 600`; verdict recorded; attributable failure blocks — else recorded as pre-existing with remediation.
- [ ] Archive with `openspec archive web-release-deploy-history --yes` (no `--skip-specs`); HANDOFF evidence + pointer handling; EXACTLY 2 commits (implementation+tests+package+spec, then HANDOFF only); no push.
