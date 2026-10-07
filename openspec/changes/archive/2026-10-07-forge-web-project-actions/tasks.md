# Tasks: forge-web-project-actions

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and both spec deltas name the same three commands
      (`feature.remove`, `feature.upgrade`, `spec.apply`) and the same
      `execution` block shape; record the language (Rust 2021 ≥1.87) and the
      `src/api/admin.rs` + `src/api/command_catalog.rs` + `frontend/` boundary.
- [x] Add `tests/forge_web_project_actions_contract.rs` skeleton reusing the
      `forge_web_command_execution_contract` harness (login, write_project,
      register_project, body_json) with the six test names from design §7.
- [x] Add a catalog serialization/agreement test skeleton (Layer C) asserting
      every `web` row has a well-formed `execution`, every non-`web` row has
      `null`, and `problems()` is empty.
- [x] Verify the implementation-handoff gate: no TBD route const, enum arm,
      parameter name or failure code left unnamed in the design.

## 2. DFS — Requirement-by-requirement implementation

- [x] `command_catalog.rs`: add the optional `execution` field to `CommandRow`
      and its serialization; add a `web_exec` constructor (extends `web_at`) that
      takes the route, method, typed `parameters`, `confirm_required`,
      `digest_bound` and emits it only for `web` rows.
- [x] `admin.rs`: export `ROUTE_ADMIN_FEATURE_REMOVE`,
      `ROUTE_ADMIN_FEATURE_UPGRADE`, `ROUTE_ADMIN_SPEC_APPLY`; widen the
      `Authoring` enum to `{FeatureAdd, FeatureRemove, FeatureUpgrade,
      SpecGenerate, SpecApply}` and parameterize `authoring_descriptor` /
      `authoring_write` over it (no parallel implementation).
- [x] `admin.rs`: implement the field gate per command — `feature.remove`
      requires non-empty `feature`; `feature.upgrade` requires non-empty `feature`
      with optional `version`; `spec.apply` requires a non-empty `findings` array
      with optional `reason`; each builds a path-free canonical descriptor and
      delegates on matching digest to `remove_feature` / `upgrade_feature` /
      `apply_routing` respectively (mirroring the existing `handle_add_feature`
      delegate path).
- [x] `mod.rs`: add the three `Route` variants, routing arms, `required_permission`
      group entries, the `admin::handle` dispatch `matches!` arm, the 404
      exhaustiveness arm and the authorization arm, mirroring
      `AdminProjectFeature`/`AdminProjectSpec` exactly.
- [x] `command_catalog.rs`: convert the `feature.remove`, `feature.upgrade` and
      `spec.apply` rows to `web_exec` rows with their routes + parameter schema;
      add `execution` blocks to the existing `feature.add` and `spec.generate`
      rows; add the three consts to `IMPLEMENTED_WEB_ROUTES`.
- [x] Implement `tests/forge_web_project_actions_contract.rs` to green: 401/415
      before core, preview-writes-nothing, wrong-digest refused, confirmed-run
      equals a direct Core call, no path/argv echo, `spec.apply` requires findings.
- [x] `frontend/index.html` + `app.js`: render a generic confirm-gated action
      control per executable row of the current project from
      `execution.parameters` (typed inputs only, no free-text shell/path field),
      driving preview → confirm → typed result through the existing request path.

## 3. BFS — Cross-surface regression and completeness

- [x] Update the in-source `web_rows_only_point_at_implemented_routes` ordered
      vec to include `feature.remove`, `feature.upgrade`, `spec.apply` at their
      build positions; run the full `cargo test` and fix any catalog parity drift.
- [x] Fix the pre-existing catalog↔Clap drift found by the full run: `identity
      change-password` and `identity generate-password` existed in the Clap tree
      but had no catalog rows, so `catalog_has_exactly_one_row_per_clap_path` was
      red on `HEAD`. Added both as truthful `cli_only` rows (a new
      `REASON_LOCAL_SECRET` for the generate-password stdout secret) and updated
      the `catalog_covers_every_clap_path` row count to 227.
- [x] Verify `GET /v1/admin/commands` and `GET /v1/admin/projects/{id}` serialize
      the new `execution` blocks and the frontend consumes them without error; no
      absolute path in any response.
- [x] Confirm the three new routes share the session/permission group and CORS
      OPTIONS wildcard with the existing admin project routes; hostile id/field
      inputs stay typed 400/404 with no echo.
- [x] Remove any current-change placeholder; confirm no Core handler, CLI
      dispatch, `CONTRACT_VERSION`, delivery or identity source file was modified
      (the catalog owner `command_catalog.rs` gained two identity rows only).

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] This change's scope green: 1213 lib passed; `--bin forge` 7/7 (including
      the two catalog parity/integrity tests); the three web contract files pass
      (`forge_web_project_actions_contract` 6,
      `forge_web_command_execution_contract` 5,
      `forge_web_command_catalog_contract` 8); `artifact_baseline_contract` 6/6
      after the pre-existing `[Unreleased]` parser fix landed alongside it.
- [x] Full `cargo test` run reached every integration target (≈1899 passed)
      because this change removed the catalog↔Clap drift that had been
      fail-stopping the suite earlier. One unrelated, pre-existing failure
      remains and is NOT this capability: `portal_browser_a11y` asserts the
      legacy server-rendered `/ui` returns 200/202/401 with the shared shell,
      but an unauthenticated `GET /ui` now 303-redirects to `/ui/sign-in`
      (introduced by the legacy-portal redirect, predates this change). It
      belongs to the separate `portal-accessible-responsive-ui` capability and
      is left for its own remediation, not silently rewritten here.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate --all --strict` 71/0.
- [x] Live Playwright run against a throwaway temp registry (never the user's
      registry): logged in, opened `verify-proj`, the five catalog-driven cards
      rendered, then preview→confirm→run for `feature.remove` (HTTP 202, manifest
      `auth` feature actually removed), `feature.upgrade` (reached Core, typed
      400 surfaced with no page crash) and `spec.apply` (HTTP 202); zero app
      console errors, zero uncaught page errors, zero network failures, and no
      absolute filesystem path in the actions UI.
