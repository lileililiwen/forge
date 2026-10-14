# Tasks: forge-api-router-table-driven

## 1. BFS baseline

- [x] Read `.ai-rules/workflow.md`, `.ai-rules/completion.md`,
      `AGENTS.md`, `.ai-rules/concerns/quality.md`,
      `.ai-rules/concerns/security.md`.
- [x] Read `src/api/router.rs` end-to-end (1006 lines: imports,
      `err_status`, `route_request`, `method_not_allowed`,
      `not_found`, `bad_request`, `required_permission`,
      `is_mutating`, `handle`, `alt_method`).
- [x] Read `src/api/model.rs:260-859` (the `Route` enum, every
      variant the cross-cutting arms classify).
- [x] Read `src/api/admin/mod.rs` and
      `src/api/admin/{agent_identity,history,creation,shipping_reads,
      assurance,identity,gate}.rs` (the beside-handler matchers that
      run before `route_request`).
- [x] Read `src/api/admin/deploy.rs:77-396` (the actual admin
      dispatch in `super::admin::handle` — the function the new
      `is_admin_dispatch` arm short-circuits to).
- [x] Skim `tests/web_assurance_browser_contract.rs` and
      `tests/web_shipping_provider_reads_contract.rs` for the test
      style (request helpers, login_token, fixture, `body_json`).
- [x] Map the four cross-cutting `match` arms to the cleanest
      combination: per-variant methods in
      `src/api/router_classify.rs` (new) called from `router.rs`.
- [x] Naming preflight: `node scripts/check-openspec-change-names.mjs`
      → PASS.

## 2. DFS requirement implementation

- [x] Author `openspec/changes/forge-api-router-table-driven/proposal.md`
      (Why, What Changes, BFS Impact Map, Capabilities, Non-goals)
      with the `source-file-size` 1006/1000 over-shoot and the
      `forge-web-gate-controls` +6-line precedent cited.
- [x] Author `openspec/changes/forge-api-router-table-driven/design.md`
      (ownership, contracts, failures, migrations) with the
      per-variant method contract and the zero-public-API-change
      invariant called out literally.
- [x] Author `openspec/changes/forge-api-router-table-driven/tasks.md`
      (this file).
- [x] Author
      `openspec/changes/forge-api-router-table-driven/specs/core-http-api/spec.md`
      delta (ADDED Requirement: "Table-driven route dispatch").
- [x] Commit 1: `openspec/changes/forge-api-router-table-driven/`
      only (`proposal: forge-api-router-table-driven`).

## 3. BFS regression/completeness

- [x] Single-change validate:
      `openspec validate forge-api-router-table-driven --strict
      --no-interactive` → PASS.
- [x] Implementation: `src/api/router_classify.rs` (new, ≤ 200 lines)
      with three exhaustive `match` methods
      (`required_permission`, `is_admin_dispatch`, `is_admin_route`)
      and one `admin_fallthrough` helper.
- [x] Implementation: `src/api/mod.rs` — one new
      `pub(super) mod router_classify;` line.
- [x] Refactor: `src/api/router.rs` —
      - delete the 138-line `required_permission` function (replace
        with `super::router_classify::required_permission` in the
        `authorize` call site, or delete the function and let
        `authorize` call the new helper directly),
      - collapse the 86-line admin pre-dispatch `matches!` arm
        (`route.rs:758-841`) to a single `if
        super::router_classify::is_admin_dispatch(&route) { ... }`,
      - collapse the 75-line final-dispatch fallthrough
        (`route.rs:924-996`) to a single
        `route => super::router_classify::admin_fallthrough(route)`.
- [x] `wc -l src/api/router.rs` ≤ 1000 after refactor (target ~915).
- [x] The new gate routes
      (`Route::AdminProjectGateStatus` / `Route::AdminProjectGateRun`)
      are dispatched by `is_admin_dispatch` exactly as the previous
      subagent's `router.rs:773-774` arm did — verified by reading
      the new code path, not by running tests.
- [x] Test: `tests/router_table_driven_contract.rs` — static-table
      coverage (every existing `Route` variant returns the same
      status code as the pre-refactor behavior) + mutation test
      (removing one `is_admin_dispatch` row causes the route to
      fall through to `not_found()`).
- [x] Commit 2: implementation + tests
      (`refactor: forge-api-router-table-driven (router stays
      ≤1000)`).

## 4. Verification

- [x] **Lightweight only** (manager runs the heavy pass at the end):
  - [x] `node scripts/check-openspec-change-names.mjs` → PASS.
  - [x] `openspec validate forge-api-router-table-driven --strict
        --no-interactive` → PASS.
  - [x] `git diff --check` → clean.
  - [x] `wc -l src/api/router.rs` ≤ 1000 (cap respected, target
        ~915).
  - [x] Source-file-size cap respected on every new/touched file
        (`router_classify.rs` ≤ 200, `router.rs` ≤ 1000).
  - [x] No `cargo build`, no `cargo test --workspace`, no `forge
        gate`, no `openspec validate --all --strict` —
        manager-owned.
- [x] **Manager-owned** (not run here):
  - [ ] `cargo build`, `cargo test --workspace` (full pre-existing
        test suite).
  - [ ] `forge gate --dry-run` rehearsal + `forge gate
        --timeout-secs 600` (with the local 9-check plan and the
        recorded verdict in HANDOFF).
  - [ ] `openspec validate --all --strict --no-interactive` (full
        active + archive + canonical spec set).
  - [ ] `openspec archive forge-api-router-table-driven --yes`
        (no `--skip-specs`; promotes the `core-http-api` delta;
        archived as `2026-10-14-forge-api-router-table-driven`).
  - [ ] Live oracle (throwaway API port, scratch `FORGE_REGISTRY`):
        `curl /healthz` → 200, `curl /v1/admin/projects/<id>/gate/status`
        anonymous → 401, with a valid admin session → 200/typed,
        `Route::AdminProjectGateRun` POST without session → 401,
        mutation test row removed → 404 — **zero
        JS/page/network errors**.
  - [ ] HANDOFF update: record the
        `forge-api-router-table-driven` deliverable + the evidence
        table; advance the existing pointer to the next active
        eligible change (or remove it if no active changes remain).
  - [ ] Commit HANDOFF; stop (no push, no further changes).
