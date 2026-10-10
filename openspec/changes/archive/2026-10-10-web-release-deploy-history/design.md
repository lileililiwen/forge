# Design: web-release-deploy-history

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library) plus `frontend/`. No sibling touched.
- **Modules changed:**
  - `src/api/admin/routes.rs` — five route constants:
    `ROUTE_ADMIN_RELEASE_HISTORY`
    (`GET /v1/admin/projects/{id}/releases`),
    `ROUTE_ADMIN_RELEASE_INSPECT`
    (`GET /v1/admin/projects/{id}/releases/{release_id}`),
    `ROUTE_ADMIN_DEPLOY_HISTORY`
    (`GET /v1/admin/projects/{id}/deploys`),
    `ROUTE_ADMIN_DEPLOY_INSPECT`
    (`GET /v1/admin/projects/{id}/deploys/{deploy_id}`),
    `ROUTE_ADMIN_DEPLOY_STATUS`
    (`GET /v1/admin/projects/{id}/deploy/status`).
    Singular `deploy/status` matches the existing `deploy/plan`
    family; plural `releases`/`deploys` match the state-dir names
    (`.forge/release/`, `.forge/deploy/`) and avoid colliding with
    the singular write/plan arms.
  - New `src/api/admin/history.rs` (or equivalent small submodule) —
    five handlers reusing `deploy_id_gate` (the shared id gate:
    hostile id → `400 admin-invalid-project-id` without echo,
    unmanaged id → `404 admin-project-unmanaged`), `guarded`
    (session gate) + `cors` + `error`/`scrub_*` from `gateway.rs`.
    Release handlers delegate to
    `release::engine::{list_releases, read_release}` after resolving
    the project dir from the registry record (same
    `Registry::open` → `inspect(id)` → `PathBuf::from(record.path)`
    path `deploy_id_gate` uses — never a browser path).
    Deploy handlers delegate to
    `deploy::engine::{list_deploys, read_deploy}` the same way.
    Status handler reads `Registry::operations_for_project(id,
    limit)` (limit from `?limit=`, default 20, clamp 1..=100,
    non-numeric → typed `400`) and applies the CLI's
    `filter_publish_deploy` kind set
    (`publish|deploy|publish.github`), returning
    `{ contract, project_id, scope, state, entries, read_only:
    true }` mirroring `forge-deploy-status/0.2.0` fields minus the
    contract rename (the envelope keeps `contract:
    API_CONTRACT_VERSION` like every admin route).
  - `src/api/model.rs` — five `Route` variants with doc comments.
  - `src/api/router.rs` — five GET arms plus OPTIONS coverage and
    `required_permission` / exhaustiveness threading (follow the
    `deploy/plan` six-segment precedent exactly).
  - `src/api/admin/deploy.rs` — dispatch arms for the five routes.
  - `src/api/command_catalog/routes.rs` — five
    `WEB_ROUTE_ADMIN_*` consts + `IMPLEMENTED_WEB_ROUTES` entries.
  - `src/api/command_catalog/rows_shipping.rs` — five leaves
    `NotYetWeb` → `web_at`/`web_exec`-free `web_at` (all Read).
  - `frontend/index.html` — one "Release & deploy history" card in
    the Delivery section reusing `#delivery-project`.
  - `frontend/app.js` — history loaders/renderers reusing
    `request`, `el`, `detailRow`, `renderErrorSummary`,
    `setResultRole` (the catalog-browser read-only pattern).
  - `tests/web_release_deploy_history_contract.rs` — new contract
    binary (§8 oracle).

## 2. Ownership and contracts

- **Owner of truth:** Core engines own the persisted shapes
  (`ReleaseListEntry`, `ReleaseState`, `DeployListEntry`,
  `DeployState`, `OperationEntry`); the handlers project them to
  path-free JSON and never invent fields.
- **Path scrub:** `DeployListEntry.state_path` is absolute and MUST
  be omitted from the list view (project it to
  `{deploy_id, project_id, target, source_revision,
  current_state, last_run_at}`). `DeployState` carries no path.
  `ReleaseListEntry`/`ReleaseState` carry no path. Error strings
  are scrubbed of the project dir via `scrub_text` (the
  `typed_deploy_error`/`typed_release_error` precedent). The
  contract test asserts the raw body never contains the project
  dir string.
- **Inspect-id validation:** `{release_id}`/`{deploy_id}` segments
  are validated before any filesystem read: empty/blank,
  `/`-bearing, or `..`-bearing values → typed `400
  admin-invalid-history-id` without echo (release and deploy share
  the helper). Unknown-but-wellformed ids → `404
  admin-history-not-found` naming only the kind (`release` /
  `deploy`), never the input.
- **Status limit:** `?limit=` parses as `usize`; absent → 20;
  clamped to 1..=100; unparseable → `400
  admin-invalid-history-limit` without echo.
- **Auth order:** origin check → session (`guarded`, 401 anonymous)
  → id gate → Core read. GET routes take no body, so no 415 arm.
- **Catalog honesty:** `rows_shipping.rs` keeps `deploy.observe`
  as `ProjectCapabilityRequired` with `REASON_PROJECT_CAPABILITY`;
  only the five pure reads convert. The `catalog_covers_every_clap_path`
  count pin stays 234; the `IMPLEMENTED_WEB_ROUTES` membership test
  gains the five routes.

## 3. Failures

| Input | Response | Side effect |
|---|---|---|
| No session | `401` via `guarded` | none |
| Wrong origin | `403 admin-origin-rejected` | none |
| Hostile `{id}` | `400 admin-invalid-project-id`, no echo | none |
| Unmanaged `{id}` | `404 admin-project-unmanaged` | none |
| Bad inspect id | `400 admin-invalid-history-id`, no echo | none |
| Unknown release/deploy id | `404 admin-history-not-found` | none |
| Bad `?limit=` | `400 admin-invalid-history-limit`, no echo | none |
| Project with no history | `200` with empty arrays / `state: "empty"` | none |
| Core read error (unreadable state dir) | mapped `err_status` + scrubbed message | none |

No failure path writes `.forge/release`, `.forge/deploy` or a
journal row; no failure path contacts a provider or adapter.

## 4. Migrations and compatibility

No registry/CLI/Core change; no response-key change on existing
routes; old frontend (unknown ids) still gets typed refusals. The
status envelope reuses the journal's `OperationEntry` serde shape
unchanged.

## 5. Alternatives rejected

- **Queue-scoped status (`--queue`) / watch mode:** needs a
  queue-id parameter with no project binding and a polling loop;
  the browser has no polling surface and the task scopes status to
  persisted per-project records. Rejected; CLI keeps it.
- **Separate history project select:** duplicates
  `#delivery-project` population and risks drift; the Delivery
  card already selects the project, so the history card reads its
  value and reports "Choose a project first" through the shared
  error summary otherwise.
- **Merging into `deploy.rs` directly:** the file is near the
  size cap; a small `history.rs` submodule keeps every file under
  the 1000-line `source-file-size` gate (the
  `portfolio_writes.rs` precedent).

## 6. Frontend pattern (reference-view reuse)

The card follows `web-project-catalog-browser` (§catalog section):
a heading + muted note ("Read-only; nothing here runs or writes"),
list/inspect/status control rows with native buttons, one
`role=status` result region per group (`history-releases-result`,
`history-deploys-result`, `history-status-result`), failures via
the Delivery `delivery-error-summary` with focus, scalar-line
rendering via `detailRow` (no `innerHTML`, no new dependency).
History buttons reuse `populateDeliveryProjects`-filled
`#delivery-project`; no second fleet fetch.

## 7. Test oracle (§8 in tasks)

Anonymous 401 on all five; hostile 400 + unmanaged 404; release
list empty → `[]` then seeded state round-trips
(`release_id/version/stage_count/last_run_at`); release inspect
found vs `404`; deploy list/inspect incl. `state_path` absence;
status empty (`state: "empty"`) then filtered (publish+deploy rows
kept, `release`/unrelated kinds dropped); limit clamp/refusal;
raw-body path-leak sweep; catalog five rows `web` + count 234;
frontend token pins (card ids, no new dep marker).
