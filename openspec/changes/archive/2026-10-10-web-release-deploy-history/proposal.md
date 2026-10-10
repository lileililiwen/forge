# Proposal: web-release-deploy-history

## Why

The dashboard operator can prepare/apply releases (`GET …/release/plan`,
`POST …/release`) and plan/apply deploys (`GET …/deploy/plan`,
`POST …/deploy`) from the browser, but cannot browse what already
happened: `release list|inspect` and `deploy list|inspect|status` are
all `not_yet_web` in the command catalog. The persisted evidence
exists — `.forge/release/<project>/<release>/state.json`,
`.forge/deploy/<project>/<deploy>/state.json`, and the registry
`operations` journal (`publish`/`deploy`/`publish.github` rows that
`forge deploy status` reads) — yet the only way to see it is the CLI.
An operator verifying a release or answering "what revision is
running?" from the dashboard must drop to a terminal.

## What Changes

- **Five read-only typed admin GET routes** (session-gated, JSON-only,
  no write, no provider, no adapter, no shell, no browser-supplied
  path):
  - `GET /v1/admin/projects/{id}/releases` — persisted release list
    via `release::engine::list_releases`.
  - `GET /v1/admin/projects/{id}/releases/{release_id}` — one
    persisted release via `release::engine::read_release`
    (absent id → typed `404`).
  - `GET /v1/admin/projects/{id}/deploys` — persisted deploy list
    via `deploy::engine::list_deploys`.
  - `GET /v1/admin/projects/{id}/deploys/{deploy_id}` — one
    persisted deploy via `deploy::engine::read_deploy`
    (absent id → typed `404`).
  - `GET /v1/admin/projects/{id}/deploy/status` — persisted
    publish/deploy journal status for that project only, via
    `Registry::operations_for_project` filtered to
    `publish|deploy|publish.github` (the CLI's
    `filter_publish_deploy` set, project-scoped; no queue/watch,
    no provider contact).
- **Catalog:** the five leaves `release.list`, `release.inspect`,
  `deploy.list`, `deploy.inspect`, `deploy.status` convert
  `NotYetWeb` → `web_at` with the typed routes above. Row count
  stays 234 (conversion, not addition). `deploy.observe` is
  unchanged (still `ProjectCapabilityRequired` — a write that
  re-runs health; explicit non-goal).
- **Frontend:** the Delivery view gains one read-only
  "Release & deploy history" card reusing the existing
  `#delivery-project` select (no new select, no new dependency):
  list buttons, inspect id inputs, status read; `role=status`
  results, error-summary focus, keyboard-operable native controls.
- **Contract tests:** new `tests/web_release_deploy_history_contract.rs`
  pins auth, id gates, empty/non-empty lists, inspect found/missing,
  status filtering, path-leak scrub, catalog pins and frontend
  token pins.

## BFS Impact Map

- **Requirements/scenarios:** `release-publishing` (release list/
  inspect reads), `adapter-deployment` (deploy list/inspect/status
  reads), `portal-web-ui` (Delivery history card). No requirement
  text changes — this closes the `not_yet_web` gap those specs
  already track via the catalog.
- **Concepts/modules:** `release::engine::{list_releases,
  read_release}`, `deploy::engine::{list_deploys, read_deploy}`,
  `Registry::{operations_for_project}`; new handlers in
  `src/api/admin/` (history submodule or alongside `release.rs`/
  `deploy.rs`); `Route` enum + router arms + dispatch; catalog
  `rows_shipping.rs` + `routes.rs`; Delivery card in
  `frontend/index.html` + `frontend/app.js`.
- **Contracts:** five new `WEB_ROUTE_ADMIN_*_HISTORY/STATUS`
  constants; `IMPLEMENTED_WEB_ROUTES` +5; catalog count pin stays
  234; `API_CONTRACT_VERSION` unchanged; every response carries
  `contract: API_CONTRACT_VERSION`.
- **Callers/persistence:** no Core/CLI/registry-schema change; reads
  open the registry read-only path and the project's state dirs;
  refused reads create no `.forge/release`, `.forge/deploy` or
  journal row.
- **Integrations:** no provider, adapter, git write, native
  toolchain or network on any path; the status read never contacts
  a provider (it answers from the journal, like the CLI's
  non-watch path).
- **Tests/compatibility:** new contract binary + regression over
  `forge_web_project_release_contract`,
  `forge_web_project_deployment_contract`,
  `forge_web_command_catalog_contract`, `portal_ui_contract`;
  old-frontend safety: unknown ids stay typed refusals.
- **Concerns:** security (id gates, no echo, path scrub, 401/415
  before Core); quality (no placeholder, no new dep, no
  `innerHTML`, file-size cap); a11y (live regions, focus,
  native controls).
- **Verification:** fmt, build, targeted + regression suites,
  names preflight, strict validate, diff-check, live
  throwaway-registry zero-JS-error proof, gate dry-run + bounded
  full gate.

## Capabilities

- Authenticated dashboard operator with a managed project.
- Read-only browser access to that project's persisted releases,
  deploys and publish/deploy journal status.

## Non-goals

- `release prepare|apply`, `deploy plan|apply|observe`, the publish
  pipeline, any provider run, any adapter run, readiness, queue
  scoping (`--queue`), watch mode, journal writes, registry schema
  change, CLI behavior change.
