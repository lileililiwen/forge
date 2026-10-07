# Tasks: Browser-executable provider publish

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the three delta specs agree, and the
      implementation-handoff gate passes (a separate agent could implement
      without inventing architecture or an owner).
- [x] Read `src/api/admin.rs` (`deploy_write`, `deploy_id_gate`,
      `authoring_digest`, `run_plan_view`/`plan_view`/`report_view`,
      `scrub_json`/`redact_local_paths`, `guarded`, `is_json`), and the release
      handlers `release_gate`/`release_request`/`release_plan`/`release_write`
      as the route template.
- [x] Read `src/publish/providers.rs` (`load_config`, `select_provider`,
      `invoke_provider`, `validate_revision`, `compose_project_name`,
      `PublishProviderRequest`, `PublishProviderResponse`, `ProviderEntry`,
      `ProviderOperation`, `PUBLISH_PROVIDER_CONTRACT`) and
      `src/main.rs::cmd_publish_provider` (6745–6855) as the exact CLI semantic
      to mirror, plus `src/registry/mod.rs::record_publish_phase` /
      `PublishPhaseEvidence` and `src/api/fleet.rs::latest_publishes` as the
      journal/projection contract.
- [x] Map each new/modified requirement and scenario to its route, gate, catalog
      row, frontend render path and test, and to its failure/boundary cases.
- [x] Add `tests/forge_web_project_publish_contract.rs` (mirrors the shipped
      release harness) with the design §7 scenarios and a hermetic `#!/bin/sh`
      stub provider staged at run time; the `FORGE_PUBLISH_PROVIDER` /
      `FORGE_PUBLISH_PROVIDER_CONFIG` variables are restored on every exit
      (env-var pollution is a documented test-harness race).
- [x] Record the exact language/toolchain (Rust 2021, `rustc 1.87` floor) and the
      project-local ownership (extend `src/api/admin.rs`, `src/api/mod.rs` and
      `src/api/command_catalog.rs`; reuse `src/publish` and `src/registry`
      unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `forge-web-project-publish` — add the read-only
      `GET /v1/admin/projects/{id}/publish/plan`: `guarded` session gate →
      `deploy_id_gate` → `FORGE_PUBLISH_PROVIDER` → provider config →
      `load_config` → `select_provider` → `git rev-parse HEAD` + `validate_revision`
      → path-free plan view + `plan_digest`; no provider invocation, no journal
      write. Register the route arm in `src/api/mod.rs`.
- [x] Add the confirm-gated `POST /v1/admin/projects/{id}/publish` on the admin
      gate: 415 JSON gate; `guarded` session gate; server-side resolution →
      digest before any provider call; no-`confirm` → 200 preview + digest, no
      write; mismatched digest → 409 `admin-digest-mismatch` + fresh digest, no
      write; matching digest + confirm → build the fixed
      `PublishProviderRequest` and `invoke_provider` → journal one `publish` row
      with the provider-reported state + phase evidence → 202 path-free typed
      result. A provider `Err` journals `failed` and returns 503
      `publish-provider-unavailable`, never success.
- [x] Add both route consts to `IMPLEMENTED_WEB_ROUTES`; add the group/CORS,
      permission, dispatch and authorize entries so the five- and six-segment
      admin publish paths share the existing `/v1/admin/projects/{id}/*`
      session, permission and CORS group and the OPTIONS wildcard.
- [x] Recatalogue in `command_catalog.rs`: the bare `publish` row from
      `provider_required`/CLI-only to a `web_exec` row (route, method POST,
      `confirm_required:true`, `digest_bound:true`, risk `remote_write`, empty
      typed-parameter list). Update the pinned `web` tuple list and
      `executable_ids` (7 → 8, adding `publish`); keep the command-count test at
      227 and relax the "every executable row has non-empty parameters" in-source
      assertion to permit the zero-parameter publish row (its exact empty
      parameter list stays pinned by the `expected` map).
- [x] Update `tests/forge_web_command_catalog_contract.rs`: add the publish
      apply route to the web allowlist and `publish` to the `web_ids` set.
- [x] Confirm `frontend/app.js::buildActionControl` renders the zero-parameter
      `execution` row generically (preview→confirm→run, no inputs, no bespoke
      code, no free-text command/path/argv field).
- [x] Implement the delta specs' scenarios alongside the code above (each
      scenario has a matching assertion in the contract test).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify the CLI `forge publish` dispatch, `cmd_publish_provider` and every
      `publish::providers` semantic are unchanged and still tested
      (`publish_contract`, `forge_web_publish_fleet_contract`).
- [x] Verify the deploy, release, authoring and portfolio-share surfaces are
      untouched and their contract tests stay green.
- [x] Verify every existing executable row (feature/spec/deploy/release actions)
      still renders and runs; the command-catalog and command-execution contract
      files pass with the new web id.
- [x] Verify no response from either new route can carry an absolute project
      path, the provider executable, the provider config path, an SSH host or a
      secret; hostile id/digest inputs stay typed 400/404/409 with no echo; a
      missing/failing provider yields an honest typed 503 + `failed` journal
      row, never a fake success.
- [x] Remove any current-change placeholder; confirm no Core publish handler,
      CLI dispatch, `CONTRACT_VERSION`, delivery or identity source file was
      modified beyond the intended `admin.rs` / `mod.rs` / `command_catalog.rs`
      / `tests/forge_web_command_catalog_contract.rs` allowlist + web-id
      surfaces.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for: `forge_web_project_publish_contract` (new),
      `forge_web_command_catalog_contract` (allowlist + `web_ids` set updated to
      include `publish`), `forge_web_command_execution_contract`,
      `forge_web_project_deployment_contract`,
      `forge_web_project_release_contract`,
      `forge_web_project_actions_contract`,
      `forge_web_project_workbench_contract`, `forge_admin_api_contract`,
      `forge_web_publish_fleet_contract`, `publish_contract`,
      `portal_ui_contract`; `cargo test --bin forge` (catalog
      integrity/coverage/`executable_ids`/route-allowlist tests) and
      `cargo test --lib api::` all pass. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict --no-interactive` 0 failures with this
      change active.
- [x] `git diff --check` clean; review newly added files (the contract test and
      the touched source files).
