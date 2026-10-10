# web-creation-catalog-browser Specification

## Purpose
Operators browse the six pure creation catalogs (profiles, features,
components, UI patterns, standards, procedures) with list, inspect
and resolve, validate structured intents, and read per-project
standard check/diff plus persisted plan receipts from the dashboard
through twenty typed read-only session-gated admin GETs that reuse
the existing Core registries (`src/api/admin/creation.rs`,
`Route::AdminCreation`); quality-gated writes, the native/file-bound
verbs and every other mutation stay CLI-only and render in the SPA
creation-catalog section as availability badges with reasons.
## Requirements
### Requirement: Typed read-only creation-catalog routes

Forge SHALL serve twenty typed read-only session-gated admin GET
routes reusing the existing Core creation registries with no new
filtering, ordering or pagination rule in any transport:
`GET /v1/admin/creation/{registry}` (list) and
`GET /v1/admin/creation/{registry}/{id}` (inspect) for `profiles`,
`features`, `components`, `ui-patterns`, `standards` and `procedures`;
`GET /v1/admin/creation/{registry}/resolve` (query-driven resolve)
for `profiles`, `features`, `components` and `ui-patterns`;
`GET /v1/admin/creation/intents/validate` (structured-intent
validation); `GET /v1/admin/projects/{id}/standard/check` and
`.../standard/diff` (snapshot reads over the server-resolved project
directory); and `GET /v1/admin/projects/{id}/intent/plans`
(persisted plan receipts, path-free). Every route SHALL be
exact-origin checked, registry read-only, journal-free,
provider-free, adapter-free, toolchain-probe-free and shell-free,
and SHALL carry both the registry contract and the admin contract
versions.

#### Scenario: Lists agree with the CLI catalogs

- **WHEN** an operator GETs `/v1/admin/creation/profiles`
  (or `features`, `components`, `ui-patterns`, `standards`,
  `procedures`)
- **THEN** the entries are byte-identical to the CLI
  `list --format json` payloads in the same stable order

#### Scenario: Inspects agree with the CLI

- **WHEN** an operator GETs `/v1/admin/creation/components/audit-action`
- **THEN** the entry is byte-identical to
  `forge component inspect audit-action --format json`

#### Scenario: Resolves are pure and reviewable

- **WHEN** an operator GETs
  `/v1/admin/creation/components/resolve?profile=rust-web&component=audit-action`
- **THEN** the outcome is byte-identical to the CLI resolve payload,
  including `rejections` when the profile refuses a component, and
  no file, registry byte or journal row is written

#### Scenario: Unknown entry stays a typed refusal

- **WHEN** an operator GETs `/v1/admin/creation/features/no-such`
- **THEN** Forge answers the typed `unknown-feature` refusal (and
  analogously `unknown-profile`, `component-invalid`,
  `ui-pattern-invalid`, `standard-invalid`, `procedure-invalid`,
  `intent-invalid`) and invents no descriptor

#### Scenario: Hostile segments never reach Core

- **WHEN** an operator GETs a creation inspect with a blank id, a
  path separator, a backslash, a `..` traversal or percent-encoding
- **THEN** Forge answers a static typed `400` (or `route-not-found`
  `404` when the traversal adds a path segment so no route matches)
  that never echoes the offending input

#### Scenario: Intent validation writes nothing

- **WHEN** an operator GETs
  `/v1/admin/creation/intents/validate?action=create_project&profile=rust-web`
- **THEN** Forge answers the validated intent plus `intent_hash`
  with no journal row (the CLI's `record_operation` line is not
  reproduced) and a malformed constraint answers `intent-invalid`

#### Scenario: Project-bound reads resolve server-side

- **WHEN** an operator GETs
  `/v1/admin/projects/alpha/standard/check`,
  `/v1/admin/projects/alpha/standard/diff?against=<pack>@<version>`
  or `/v1/admin/projects/alpha/intent/plans`
- **THEN** the project directory is resolved server-side from the
  validated id (the browser sends no path), absolute paths are
  scrubbed from reports, plan receipts carry plan ids only, and an
  unknown project answers typed `unknown-project`

#### Scenario: Reads write nothing

- **WHEN** any of the twenty routes runs
- **THEN** no registry byte, table row, journal row, fixture file or
  receipt is written

### Requirement: Creation command rows served from the browser

The twenty command-catalog rows `profile.list|inspect|resolve`,
`feature.list|inspect|resolve`, `component.list|inspect|resolve`,
`ui-pattern.list|inspect|resolve`, `standard.list|inspect|check|
diff`, `procedure.list|inspect` and `intent.validate|list` SHALL be
`web` with the typed routes above (risk `read`, capabilities kept
verbatim). The catalog count SHALL stay 234 (conversion, not
addition) and `problems()` SHALL stay empty. `profile.preflight`,
`procedure.validate`, `component.qualify`, `ui-pattern.install`,
`standard.upgrade` and every other write SHALL NOT change state.

#### Scenario: Gap rows resolve to implemented routes

- **WHEN** the catalog is enumerated
- **THEN** each of the twenty rows carries its admin route, the
  route is in `IMPLEMENTED_WEB_ROUTES`, and no other row changes
  state

### Requirement: Read-only creation-catalog browser section

The standalone SPA SHALL render a read-only creation-catalog section
on the projects view (after the project-catalog browser) with a
registry picker, a search box, per-registry list/inspect/
resolve-or-check/diff rendering, availability badges with reasons
for the verbs that stay CLI-only, and honest unavailable-with-reason
states. The section SHALL use explicit reads only, native controls,
`role="status"` results with error-summary focus, `textContent`-only
rendering, no new dependency, no shell, no path submission and no
executable control.

#### Scenario: Operator browses without the terminal

- **WHEN** a signed-in operator opens the projects view, picks the
  components registry and lists, inspects `audit-action` and resolves
  it for `rust-web`
- **THEN** each result renders the Core payload with zero JS console
  errors and no page reload

#### Scenario: CLI-only leftovers stay honest

- **WHEN** the section renders the creation remainder
- **THEN** `component qualify`, `ui-pattern install`, `standard
  upgrade`, `profile preflight` and `procedure validate` show their
  catalog availability badges, plain-language reasons and exact
  terminal commands — never an executable control

