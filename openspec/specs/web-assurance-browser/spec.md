# web-assurance-browser Specification

## Purpose
Operators browse the seventeen assurance reads (spec list/inspect/route,
remediate scan/diff, describe/classify list/show, contract
list/inspect/emit, governance list/status/inspect, analytics metrics,
studio preview read) from the dashboard through typed read-only
session-gated admin GETs that reuse the existing Core stores
(`src/api/admin/assurance.rs` on the shared `Route::AdminCreation`
triple); writes, provider probes, adapter runs and every other mutation
stay CLI-or-existing-web and render in the SPA assurance section as
availability badges with reasons.
## Requirements
### Requirement: Typed read-only assurance routes

Forge SHALL serve seventeen typed read-only session-gated admin GETs
reusing the existing Core stores with no new filtering, ordering or
pagination rule in any transport: `GET /v1/admin/contracts` (list) and
`GET /v1/admin/contracts/{family}` (inspect) for the vendored contract
catalog; `GET /v1/admin/projects/{id}/specs` (list),
`GET .../specs/{spec}` (inspect) and `GET .../spec/route?finding=`
(route) for generated specs; `GET .../remediate/scan` (scan) and
`GET .../remediate/diff?finding=&pack=` (diff via server-side plan
rebuild); `GET .../describe/proposals` (list) and
`GET .../describe/proposals/{proposal}` (show);
`GET .../classify/proposals` (list) and
`GET .../classify/proposals/{proposal}` (show);
`GET .../contracts/emit?family=` (project a Core record into a platform
envelope); `GET .../governance` (list),
`GET .../governance/status` and `GET .../governance/inspect` (read-only
observation); `GET .../analytics/metrics?window_days=` (registry-only
metrics); and `GET .../studio/preview` (current session envelope).
Every route SHALL be exact-origin checked, registry read-only,
journal-free, provider-probe-free, adapter-free, shell-free and
browser-path-free, and SHALL carry both the registry contract and the
admin contract versions.

#### Scenario: Spec lists agree with the CLI

- **WHEN** an operator GETs `/v1/admin/projects/alpha/specs`
- **THEN** the entries are byte-identical to `forge spec list --format
  json` payloads in the same stable order, with absolute paths scrubbed

#### Scenario: Spec inspects agree with the CLI

- **WHEN** an operator GETs `/v1/admin/projects/alpha/specs/<id>`
- **THEN** the draft is byte-identical to `forge spec inspect <id>
  --format json`; an unknown id answers typed `unknown-spec` and a
  hostile segment answers a static typed `400` without echo

#### Scenario: Spec routes classify without applying

- **WHEN** an operator GETs
  `/v1/admin/projects/alpha/spec/route?finding=<finding>`
- **THEN** the decision is byte-identical to `forge spec route --finding
  <finding> --format json` and no file, registry byte or journal row is
  written

#### Scenario: Remediate scan and diff are read-only

- **WHEN** an operator GETs `/v1/admin/projects/alpha/remediate/scan`
  or `/v1/admin/projects/alpha/remediate/diff?finding=<f>&pack=<p>@<v>`
- **THEN** the scan report and the diff entries are byte-identical to
  the CLI `scan` / `diff` payloads for the same finding and pack, and
  no file, registry byte or journal row is written

#### Scenario: Describe and classify proposals render

- **WHEN** an operator GETs `/v1/admin/projects/alpha/describe/proposals`
  (or `classify/proposals`) and then
  `.../describe/proposals/<proposal>` (or `classify` equivalent)
- **THEN** the entries and manifests are byte-identical to `forge
  describe list|show` / `forge classify list|show --format json`; an
  unknown proposal answers typed `unknown-proposal`

#### Scenario: Contract list, inspect and emit are read-only

- **WHEN** an operator GETs `/v1/admin/contracts`,
  `/v1/admin/contracts/<family>` and
  `/v1/admin/projects/alpha/contracts/emit?family=<family>`
- **THEN** the list matches `forge contract list --format json`, the
  inspect carries the vendored schema `required` projection, and the
  emit projects the server-resolved project record; an unknown family
  answers a static typed `400` without echo and an empty readiness
  source answers honest `unavailable-with-reason`

#### Scenario: Governance reads never run or persist

- **WHEN** an operator GETs `/v1/admin/projects/alpha/governance`,
  `.../governance/status` or `.../governance/inspect`
- **THEN** the provider list matches `forge governance list` and the
  observation matches the read-only `evaluate_project` projection (never
  `check_project`); an enabled external provider answers honest
  `unavailable-with-reason` naming the provider and no adapter runs, no
  observation is persisted, no journal row is written

#### Scenario: Analytics metrics probe nothing and write nothing

- **WHEN** an operator GETs
  `/v1/admin/projects/alpha/analytics/metrics` (optionally
  `?window_days=N`)
- **THEN** the report is the registry-only
  `aggregate_project_metrics` projection with empty externals; no
  provider is probed, no summary file is written, no journal row is
  written; an unparsable window answers typed `400` and the registry
  bytes are identical before and after the read

#### Scenario: Studio preview reads without spawning

- **WHEN** an operator GETs `/v1/admin/projects/alpha/studio/preview`
- **THEN** the envelope is byte-identical to the CLI `studio preview`
  status read for the persisted session (or `state: none` when no
  session exists); no runner spawns, no port binds, no journal row is
  written beyond what the status read already does (none)

#### Scenario: Hostile and unknown inputs stay typed

- **WHEN** an operator GETs any project-bound assurance route with a
  blank id, a path separator, a backslash, a `..` traversal or
  percent-encoding, or names an unmanaged project
- **THEN** Forge answers a static typed `400` (or route `404` when the
  traversal adds a segment so no route matches, `unknown-project`
  `404` for unmanaged ids) that never echoes the offending input

### Requirement: Assurance SPA section

The projects view SHALL render one read-only "Assurance" section after
the creation-catalog card that browses the seventeen reads with explicit
operator actions and honest unavailable-with-reason states.

#### Scenario: Per-registry rendering with honest gaps

- **WHEN** a signed-in operator opens `/projects`, picks a registry
  (spec / remediate / describe / classify / contract / governance /
  analytics / studio) and issues an explicit list/inspect/route-or-scan-
  or-diff-or-emit-or-metrics-or-preview read
- **THEN** the SPA renders the typed payload (or the honest
  unavailable-with-reason), never issues a write, never auto-loads, and
  the CLI-only remainder (`describe suggest|approve|reject`, `classify
  suggest|derive`, `contract validate`, `governance use`, `analytics
  inspect`, `studio spec|refine`) renders as availability badges with
  reasons derived from the loaded command catalog

#### Scenario: Accessible read-only interaction

- **WHEN** the operator drives the assurance section by keyboard with a
  screen reader
- **THEN** every control is a native element, results carry
  `role="status"`, failures move focus to the error summary, text
  scales at the 12px floor with no new motion or dependency, and no
  markup is built with `innerHTML`
