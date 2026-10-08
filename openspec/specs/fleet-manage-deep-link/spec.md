# fleet-manage-deep-link Specification

## Purpose
TBD - created by archiving change fleet-manage-deep-link. Update Purpose after archive.
## Requirements
### Requirement: A managed project's fleet action boots that project in the workbench

A managed (or Forge-self) project's fleet row action SHALL navigate to
`/workbench?project=<identity>` with the row's project identity
URL-encoded in the query string. Loading `/workbench?project=<id>`
(whether by click, paste, reload or back/forward) SHALL select that
project in the workbench project selector and load its detail once the
fleet data is available. The workbench SHALL NOT auto-load anything for a
bare `/workbench` URL, and SHALL NOT alter the current selection or loaded
detail when no `project` parameter is present.

#### Scenario: Managed project opens its deep-linkable workbench URL

- **WHEN** the operator activates the row action for a managed project
- **THEN** the URL becomes `/workbench?project=<identity>` and that
  project's workbench detail is loaded

#### Scenario: Workbench deep link boots on direct load and reload

- **WHEN** the operator loads `/workbench?project=<id>` directly, or
  reloads it
- **THEN** the workbench view is visible, the selector names that
  project, and its detail (lifecycle, manifest, health) is loaded

#### Scenario: Workbench deep link follows back/forward

- **WHEN** the operator visits `/workbench?project=<a>` then
  `/workbench?project=<b>`, then goes Back, then Forward
- **THEN** Back loads project `<a>` and Forward loads project `<b>` again

#### Scenario: Unknown workbench project id is safe

- **WHEN** the operator loads `/workbench?project=no-such-project`
- **THEN** no project is loaded or selected by the link, and a notice
  names the id and states that nothing was loaded

### Requirement: A stale management link for a managed project reaches the workbench

Loading `/management?project=<id>` where the workspace candidate for
`<id>` is already `registered` SHALL navigate to
`/workbench?project=<id>` instead of reporting that the project cannot be
onboarded. Other non-selectable states render inside the project-scoped
card per the project-scoped requirement below.

#### Scenario: Registered id on the management view redirects

- **WHEN** the operator loads `/management?project=<registered-id>`
- **THEN** the URL becomes `/workbench?project=<registered-id>` and that
  project's workbench detail is loaded

### Requirement: The login round-trip preserves the deep link

An unauthenticated dashboard load SHALL redirect to
`login.html?next=<current path plus query>`. After a successful sign-in
(and when the login page loads with an existing session), the browser
SHALL return to `next` when it is a same-origin path on the dashboard
route allowlist, and to `index.html` otherwise. A `next` value that is
absolute, cross-origin, unknown, or the login page itself SHALL be
ignored and SHALL fall back to `index.html`; it SHALL never navigate
off-origin.

#### Scenario: Signed-out deep link boots its project after sign-in

- **WHEN** the operator opens `/management?project=<id>` (or
  `/workbench?project=<id>`) with no session and signs in
- **THEN** the browser returns to that exact URL and the destination
  boots that project (project-scoped management card, or workbench
  detail loaded)

#### Scenario: Hostile next value is refused

- **WHEN** the login page carries `?next=https://example.invalid/` (or any
  off-allowlist path)
- **THEN** sign-in lands on `index.html` and no off-origin navigation
  occurs

### Requirement: A project management deep link renders the project-scoped view first

Loading `/management` with the `project` query key present SHALL render
the project-scoped onboarding card for that id as the primary content —
a candidate summary plus only that project's import/register
preview/confirm/run actions — and SHALL hide the global bulk workspace
table. Loading plain `/management` (no `project` key) SHALL render the
global bulk onboarding view exactly as before. An onboardable candidate
SHALL offer the scoped preview; an already-`registered` id SHALL redirect
to `/workbench?project=<id>`; an unknown, empty, or otherwise
non-selectable id SHALL render a safe notice naming the id with no
selection and no preview. Reload, back/forward, and the login `?next=`
round-trip SHALL reconcile the scoped view with the URL.

#### Scenario: Deep link leads with the scoped card, bulk hidden

- **WHEN** the operator loads `/management?project=<onboardable-id>`
- **THEN** the scoped card is visible and names that project, the bulk
  table is hidden, no bulk row is ticked, and the scoped preview covers
  only that project

#### Scenario: Plain management stays global bulk

- **WHEN** the operator loads `/management` with no `project` key
- **THEN** the bulk table is visible and the scoped card is hidden

#### Scenario: Unknown or empty project id is safe

- **WHEN** the operator loads `/management?project=no-such-project`
  (or a present-but-empty `?project=`)
- **THEN** the scoped safe notice names the id (or the missing id) with
  "nothing was selected", no bulk row is ticked, and no preview is
  offered

