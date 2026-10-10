# web-project-catalog-browser (delta)

## ADDED Requirements

### Requirement: Typed read-only admin catalog routes

Forge SHALL serve six typed read-only admin GET routes reusing the
existing Core catalog, gaps and fleet services with no new filtering,
ordering or pagination rule in any transport: `GET
/v1/admin/projects/catalog` (paginated list),
`GET /v1/admin/projects/{id}/catalog` (all records for one project),
`GET /v1/admin/projects/catalog/tags` and `.../languages` (distinct
values with counts over the filtered catalog),
`GET /v1/admin/projects/catalog/gaps` (evidence-backed findings with
category/status/remediation-class filters), and
`GET /v1/admin/fleet/{entry}` (one fleet registry entry). Every route
SHALL be session-gated (401 without), exact-origin checked, registry
read-only, journal-free, provider-free and shell-free.

#### Scenario: List and inspect agree with the existing catalog family

- **WHEN** an operator GETs `/v1/admin/projects/catalog` or
  `/v1/admin/projects/{id}/catalog` with the same filters as
  `GET /v1/projects/catalog`
- **THEN** the records, order and pagination bytes are identical and
  every record carries `source`, `source_revision`, `observed_at` and
  `freshness`

#### Scenario: Unknown project stays a typed refusal

- **WHEN** an operator GETs `/v1/admin/projects/no-such/catalog`
- **THEN** Forge answers the typed `unknown-project` refusal and
  invents no record

#### Scenario: Invalid filter stays a typed refusal

- **WHEN** an operator supplies an unknown filter key, category, status
  or remediation class
- **THEN** Forge answers typed `catalog-invalid` and changes nothing

#### Scenario: Reads write nothing

- **WHEN** any of the six routes runs
- **THEN** no registry byte, table row or journal row is written

### Requirement: Catalog command rows served from the browser

The six command-catalog rows `fleet.inspect`, `project.list`,
`project.inspect`, `project.tags`, `project.languages` and
`project.gaps` SHALL be `web` with the typed routes above (risk
`read`, keep `registry_read` caps). The catalog count SHALL stay 234
(conversion, not addition) and `problems()` SHALL stay empty. `fleet
online`, `project github *` and `inventory show` SHALL NOT change.

#### Scenario: Gap rows resolve to implemented routes

- **WHEN** the catalog is enumerated
- **THEN** each of the six rows carries its admin route, the route is
  in `IMPLEMENTED_WEB_ROUTES`, and no other row changes state

### Requirement: Dashboard exposes a read-only catalog browser

The dashboard's projects view SHALL carry a read-only catalog-browser
section (after the command reference, before the workbench) with:
per-project inspect rendering all sources with provenance; tags and
languages tables with counts; a gaps list rendering `id`,
`project_id`, `category`, `status`, `remediation_class` and evidence
(`source`, `source_revision`, `observed_at`) plus the summary line;
and a fleet-entry inspect. Reads SHALL reuse `GET
/v1/projects/catalog` where it suffices and the six admin routes where
no route exists. No write, provider call, shell, path submission or
new frontend dependency SHALL be added; all strings SHALL render as
text; results SHALL be `role="status"` live regions with honest
unavailable/empty/error states.

#### Scenario: Operator inspects one project across all sources

- **WHEN** the operator picks a project and runs inspect
- **THEN** every source's record renders with its provenance and
  freshness, or a typed error renders when the id is unknown

#### Scenario: Operator reads tags, languages and gaps

- **WHEN** the operator loads tags, languages and gaps
- **THEN** each tag/language renders with its count and each gap
  renders with its verdict and evidence; empty sets render explicit
  empties, never silence

#### Scenario: Unauthenticated browser sees no catalog

- **WHEN** the browser calls any of the six routes without a session
- **THEN** it is redirected to login and the section shows the
  unavailable state
