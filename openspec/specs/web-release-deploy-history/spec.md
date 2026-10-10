# web-release-deploy-history Specification

## Purpose
Let dashboard operators browse persisted release/deploy history without the CLI: five read-only admin GETs over the existing Core release/deploy stores and the registry journal (no writes, no provider, no adapter), with the five catalog leaves converted to `web` and a read-only Delivery history card.
## Requirements
### Requirement: Read-only release history in the browser

Forge SHALL serve the persisted per-project release list and single
release inspect through session-gated admin GET routes reusing
`release::engine::list_releases` and `read_release`. Reads SHALL
never write state, run an adapter, mutate git, or contact a
provider.

#### Scenario: Release list reads back persisted releases

- **WHEN** an operator GETs
  `/v1/admin/projects/{id}/releases` for a project with persisted
  releases
- **THEN** the response carries one entry per persisted release
  (`release_id`, `version`, `stage_count`, `last_run_at`) and a
  project with no releases gets `200` with an empty list

#### Scenario: Release inspect resolves or 404s honestly

- **WHEN** an operator GETs
  `/v1/admin/projects/{id}/releases/{release_id}`
- **THEN** a persisted release returns its stored state and an
  unknown id returns typed `404` with nothing written

### Requirement: Read-only deploy history in the browser

Forge SHALL serve the persisted per-project deploy list and single
deploy inspect through session-gated admin GET routes reusing
`deploy::engine::list_deploys` and `read_deploy`, with the absolute
`state_path` omitted from every response. Reads SHALL never write
state, run an adapter, or contact a provider.

#### Scenario: Deploy list and inspect stay path-free

- **WHEN** an operator GETs `/v1/admin/projects/{id}/deploys`
  and `/v1/admin/projects/{id}/deploys/{deploy_id}`
- **THEN** entries carry `deploy_id`, `target`, `current_state`
  and `last_run_at` with no absolute path in the raw body, and an
  unknown deploy id returns typed `404` with nothing written

### Requirement: Read-only deploy status in the browser

Forge SHALL serve `GET
/v1/admin/projects/{id}/deploy/status` from the persisted registry
journal only (`operations_for_project` filtered to
`publish|deploy|publish.github`, `?limit=` default 20 clamped to
1..=100). It SHALL never contact a provider, run an adapter, or
offer queue scoping or watch mode.

#### Scenario: Status answers from persisted rows

- **WHEN** an operator GETs the status route after publish/deploy
  rows were journaled alongside unrelated rows
- **THEN** the response carries only the publish/deploy rows for
  that project with `read_only: true`, and a project with no such
  rows gets `state: "empty"`

### Requirement: History routes share the admin read boundary

All five history routes SHALL refuse anonymous requests with `401`
before any Core call, refuse hostile project ids with typed `400`
without echo, refuse unmanaged ids with typed `404`, refuse bad
inspect ids and bad limits with typed `400` without echo, and
SHALL never create `.forge/release`, `.forge/deploy` or journal
rows on any refused or read path.

#### Scenario: Refused reads change nothing

- **WHEN** an operator hits any history route anonymously, with a
  hostile or unmanaged id, a bad inspect id, or a bad limit
- **THEN** Forge answers the typed refusal and the state dirs and
  journal are identical before and after

### Requirement: Catalog names the history routes as web

The catalog SHALL report `release.list`, `release.inspect`,
`deploy.list`, `deploy.inspect` and `deploy.status` as `web` with
their typed routes, keep `deploy.observe` out of the browser, and
keep the total row count at 234.

#### Scenario: History leaves convert without addition

- **WHEN** an operator reads the command catalog
- **THEN** the five history rows carry `availability: "web"` with
  the exact new routes and the row count is unchanged

### Requirement: Delivery history card

The Delivery view SHALL render a read-only "Release & deploy
history" card reusing the existing project select: list buttons,
inspect id inputs and a status read over the five routes, with
`role=status` results, error-summary focus and native controls.
No control SHALL write, run a provider, or add a frontend
dependency.

#### Scenario: Operator browses history without the CLI

- **WHEN** an operator picks a project and drives the history
  card (lists, an inspect, the status read)
- **THEN** each result renders the persisted records, failures
  move focus to the error summary, and no request outside the
  five read routes is issued

