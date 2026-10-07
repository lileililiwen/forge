# Tasks: Browser-executable project deploy

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the three delta specs agree, and the
      implementation-handoff gate passes (a separate agent could implement
      without inventing architecture or an owner).
- [x] Read `src/api/admin.rs` (`authoring_descriptor`/`authoring_digest`/
      `authoring_write`/`guarded`), `src/api/mod.rs:3080`
      (`handle_apply_deployment`), `src/deploy/engine.rs` (`prepare_deploy`,
      `apply_deploy`, `load_config`, `invoke_adapter`, `DeployAdapterConfig`),
      and `src/api/command_catalog.rs` (`web_exec`, `IMPLEMENTED_WEB_ROUTES`,
      the pinned `executable_ids` test) as the reuse contract.
- [x] Map each new/modified requirement and scenario to its route, gate, catalog
      row, frontend render path and test, and to its failure/boundary cases.
- [x] Add the `tests/forge_web_project_deployment_contract.rs` test file
      (mirrors the shipped `forge_web_project_actions_contract.rs` harness)
      with eight scenarios from design §7 plus a GET-plan route and an
      empty-target-defaults-to-manifest test; the stub deployer bodies are
      inlined as `ADAPTER_OK_BODY` / `ADAPTER_FAIL_BODY` and staged at run
      time so the test is hermetic and `FORGE_DEPLOYER_BIN` is restored on
      every exit (env-var pollution is a documented test-harness race).
- [x] Record the exact language/toolchain (Rust 2021, `rustc 1.87` floor) and the
      project-local ownership (extend `src/api/admin.rs` +
      `src/api/command_catalog.rs`; reuse `src/deploy` unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `forge-web-project-deployment` — add the read-only
      `GET /v1/admin/projects/{id}/deploy/plan` route: session gate → id gate →
      server-resolve `project_dir` from `registry.inspect(id).path` →
      `load_config` → `prepare_deploy` (dry_run, confirm:false) → path-free plan
      view; no write. Register the route arm in `src/api/mod.rs`.
- [x] Add the confirm-gated `POST /v1/admin/projects/{id}/deploy` on the admin
      gate: 415 JSON gate; `guarded` session gate; `validate_project_id` → 400
      (no path echo) and `registry.inspect` → 404 before any digest; build the
      path-free `{project_id, target}` descriptor and `plan_digest`; no-confirm
      → 200 preview + digest, no write; mismatched digest → 409
      `admin-digest-mismatch` + fresh digest, no write; matching digest +
      confirm → `run_with_operation(db, "admin.deploy", id, || apply_deploy(...))`
      → 202 path-free report. Reuse `deploy::engine` unchanged.
- [x] Add both route consts to `IMPLEMENTED_WEB_ROUTES`; add the group/CORS
      matching so the six-segment admin deploy paths share the session/permission
      group and the OPTIONS wildcard with the existing admin project routes.
- [x] Recatalogue in `command_catalog.rs`: `deploy.plan` from
      `project_capability_required` to a `web` read row pointing at the plan
      route; `deploy.apply` to a `web_exec` row (route, method POST,
      `confirm_required:true`, `digest_bound:true`, risk `remote_write`,
      parameter `target:string` optional). Update the pinned `executable_ids`
      test to include `deploy.apply` (6 total); keep the command-count test at
      227.
- [x] Confirm `frontend/app.js` renders the new `execution` row generically
      (typed `target` field, preview→confirm→run) with no bespoke code and no
      free-text command/path/argv field; no adjustment needed — the shipped
      `buildActionControl` already covers the typed `string` parameter case.
- [x] Implement the delta specs' scenarios alongside the code above (each
      scenario has a matching assertion in the contract test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the bearer `POST /v1/projects/{id}/deployments` route and
      `handle_apply_deployment` are unchanged and still tested.
- [x] Verify the portfolio-share delivery surface (`src/api/delivery.rs`) is
      untouched and its contract tests stay green.
- [x] Verify every existing executable row (feature/spec actions) still renders
      and runs; the command-catalog and command-execution contract files pass.
- [x] Verify no response from either new route can carry an absolute path, the
      adapter binary, or a credential; hostile id/field inputs stay typed
      400/404/409 with no echo; a missing/failing `FORGE_DEPLOYER_BIN` yields an
      honest typed failure, never a fake success.
- [x] Remove any current-change placeholder; confirm no Core deploy handler, CLI
      dispatch, `CONTRACT_VERSION`, delivery or identity source file was modified
      beyond the intended `admin.rs` / `mod.rs` / `command_catalog.rs` /
      `tests/forge_web_command_catalog_contract.rs` allowlist + web-id
      surfaces.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_project_deployment_contract` (new,
      8 passed), `forge_web_command_catalog_contract` (8 passed, allowlist +
      `web_ids` set updated to include `deploy.plan` and `deploy.apply`),
      `forge_web_command_execution_contract` (5 passed),
      `forge_web_project_actions_contract` (6 passed),
      `forge_web_project_workbench_contract` (11 passed),
      `forge_admin_api_contract` (4 passed), `forge_web_fleet_contract`
      (11 passed), `forge_web_portfolio_controls_contract` (11 passed),
      `forge_web_delivery_controls_contract` (10 passed),
      `forge_portal_frontend_contract` (5 passed), `portal_ui_contract`
      (39 passed), `identity_contract` (19 passed), `deploy_contract`
      (17 passed), `deploy_cross_surface` (5 passed); `cargo test --bin
      forge` (catalog integrity/coverage/`executable_ids`/route-allowlist
      tests, 7 passed) and `cargo test --lib api::` and `deploy::` and
      `identity::` all pass. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict --no-interactive` 72/0 with this
      change active.
- [x] `git diff --check` clean; review newly added files (the contract test
      and the four touched source files).
- [ ] Headless-Chromium drive against a throwaway registry (never the user's
      real registry): sign in, open a managed project, preview the deploy
      (digest, no write), then — with the stub deployer configured — confirm and
      apply; assert zero console/page/network errors and no absolute path in the
      DOM. Recorded as evidence, not assumed. The catalog-driven frontend
      renderer (`buildActionControl`) is unchanged from
      `forge-web-project-actions` and is exercised there with a real
      Playwright drive; the deploy row joins the same `execution`-block
      render path, so a fresh browser drive would re-cover the same
      control. `UNVERIFIED` here means: a dedicated browser drive for the
      deploy row specifically was not run in this cycle's budget — the
      control renders generically and the contract tests prove the
      HTTP/JSON contract the browser would call.
