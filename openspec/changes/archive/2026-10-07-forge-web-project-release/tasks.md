# Tasks: Browser-executable project release

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the three delta specs agree, and the
      implementation-handoff gate passes (a separate agent could implement
      without inventing architecture or an owner).
- [x] Read `src/api/admin.rs` (`deploy_write`, `deploy_id_gate`,
      `authoring_digest`, `run_plan_view`/`plan_view`/`report_view`,
      `scrub_json`/`redact_local_paths`), `src/release/engine.rs`
      (`prepare_release`, `apply_release`, `PlanReport`, `ReleaseReport`,
      `StageOutcome`, `effective_stages`) and `src/release/mod.rs`
      (`ReleaseRequest`, `ReleaseConfig`, `Semver`,
      `release_config_from_manifest`, `ReleaseAdapterConfig::from_env`,
      `load_changelog`), plus `src/api/command_catalog.rs` (`web_at`, `web_exec`,
      `IMPLEMENTED_WEB_ROUTES`, the pinned tests) as the reuse contract.
- [x] Map each new/modified requirement and scenario to its route, gate, catalog
      row, frontend render path and test, and to its failure/boundary cases.
- [x] Add the `tests/forge_web_project_release_contract.rs` test file (mirrors
      the shipped deployment harness) with the design §7 scenarios; the stub
      adapters are staged at run time so the test is hermetic and the
      `FORGE_*_BIN` variables are restored on every exit (env-var pollution is a
      documented test-harness race).
- [x] Record the exact language/toolchain (Rust 2021, `rustc 1.87` floor) and the
      project-local ownership (extend `src/api/admin.rs` +
      `src/api/command_catalog.rs`; reuse `src/release` unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `forge-web-project-release` — add the read-only
      `GET /v1/admin/projects/{id}/release/plan` route: session gate → id gate →
      parse/normalize `version` → `release_config_from_manifest` →
      `prepare_release` (dry_run, confirm:false, `config.stages`) → path-free plan
      view + `plan_digest`; no write. Register the route arm in `src/api/mod.rs`.
- [x] Add the confirm-gated `POST /v1/admin/projects/{id}/release` on the admin
      gate: 415 JSON gate; `guarded` session gate; `validate_project_id` → 400
      (no path echo), `registry.inspect` → 404 and `version` parse → 400 before
      any digest; build the path-free `{project_id, version}` descriptor and
      `plan_digest`; no-confirm → 200 preview + digest, no write; mismatched
      digest → 409 `admin-digest-mismatch` + fresh digest, no write; matching
      digest + confirm → `run_with_operation(db, "release", id, || apply_release(...))`
      → 202 path-free report. Reuse `release::engine` unchanged.
- [x] Add both route consts to `IMPLEMENTED_WEB_ROUTES`; add the group/CORS,
      permission, dispatch and authorize entries so the five- and six-segment
      admin release paths share the session/permission group and the OPTIONS
      wildcard with the existing admin project routes.
- [x] Recatalogue in `command_catalog.rs`: `release.prepare` from
      `not_yet_web` to a `web` read row pointing at the plan route;
      `release.apply` to a `web_exec` row (route, method POST,
      `confirm_required:true`, `digest_bound:true`, risk `remote_write`,
      parameter `version:string` required). Update the pinned `web` tuple list and
      `executable_ids` (6 → 7, adding `release.apply`); keep the command-count
      test at 227.
- [x] Confirm `frontend/app.js` renders the new `execution` row generically (a
      required typed `version` field, preview→confirm→run) with no bespoke code
      and no free-text command/path/argv field.
- [x] Implement the delta specs' scenarios alongside the code above (each
      scenario has a matching assertion in the contract test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the CLI `forge release` dispatch and every `release::engine`
      semantic are unchanged and still tested (`release_contract`,
      `release_cross_surface`).
- [x] Verify the deploy, authoring and portfolio-share delivery surfaces are
      untouched and their contract tests stay green.
- [x] Verify every existing executable row (feature/spec/deploy actions) still
      renders and runs; the command-catalog and command-execution contract files
      pass with the two new web ids.
- [x] Verify no response from either new route can carry an absolute path, the
      adapter binaries, a git remote credential or a secret; hostile
      id/version/digest inputs stay typed 400/404/409 with no echo; a
      missing/failing adapter or `origin` yields an honest typed failure, never a
      fake success.
- [x] Remove any current-change placeholder; confirm no Core release handler, CLI
      dispatch, `CONTRACT_VERSION`, delivery or identity source file was modified
      beyond the intended `admin.rs` / `mod.rs` / `command_catalog.rs` /
      `tests/forge_web_command_catalog_contract.rs` allowlist + web-id surfaces.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_project_release_contract` (new),
      `forge_web_command_catalog_contract` (allowlist + `web_ids` set updated to
      include `release.plan` and `release.apply`),
      `forge_web_command_execution_contract`,
      `forge_web_project_deployment_contract`,
      `forge_web_project_actions_contract`,
      `forge_web_project_workbench_contract`, `forge_admin_api_contract`,
      `forge_web_fleet_contract`, `forge_web_portfolio_controls_contract`,
      `forge_web_delivery_controls_contract`, `forge_portal_frontend_contract`,
      `portal_ui_contract`, `catalog_contract`, `release_contract`,
      `release_cross_surface`; `cargo test --bin forge` (catalog
      integrity/coverage/`executable_ids`/route-allowlist tests) and
      `cargo test --lib api::` all pass. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict --no-interactive` 0 failures with this
      change active.
- [x] `git diff --check` clean; review newly added files (the contract test and
      the touched source files).
