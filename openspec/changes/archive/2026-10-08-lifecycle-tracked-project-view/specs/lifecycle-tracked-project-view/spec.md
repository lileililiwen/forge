# lifecycle-tracked-project-view (delta)

## REMOVED Requirements

### Requirement: Standalone Command catalog (reference) page
- **Driver:** `forge-web-command-catalog` (browser page; the JSON
  endpoint behind the same name stays)
- **Surfaces removed:**
  - The `<section id="commands">` block in `frontend/index.html`.
  - The "Commands" nav link.
  - The catalog page's table body, search input, category
    filter, availability filter, "no results" notice, and
    count line.
  - The `renderCommands` and `showCommandsError` functions in
    `frontend/app.js` and the catalog page's filter event
    handlers.
  - The catalog-page styles (`.command-table`, `.command-id`,
    `.command-path`, `.command-guidance`, `.depth-1`,
    `.depth-2`) in `frontend/styles.css`.
- **Verification oracle:** the named test
  `frontend_uses_the_catalog_for_per_project_buttons_not_a_reference_page`
  in `tests/forge_web_command_catalog_contract.rs` is the
  new shape of the contract; the old test
  `frontend_ships_the_catalog_as_labels_never_execution` is
  removed.

## ADDED Requirements

### Requirement: Per-project workbench is lifecycle-tracked
Forge SHALL show, when an operator opens one managed project
in the workbench, the project's current maturity, target
maturity, profile, path, last update, and the typed
lifecycle commands that apply to that project, in that
order. The maturity level (`L0` … `L4`) is the same level
the project manifest carries; the operator-visible
description of the level is one short line, not a
narrative.

#### Scenario: Operator opens a project
- **WHEN** the operator selects a managed project in the
  workbench dropdown and clicks "Open project"
- **THEN** the workbench shows the lifecycle card first
  (maturity, target, profile, path, last update), then the
  "Manage this project" card with one button per available
  action, then the upgrade workflow card, then the
  read-only cards (manifest, health, status, delivery,
  journal evidence)

#### Scenario: Lifecycle card reads the manifest
- **WHEN** the workbench opens
- **THEN** the lifecycle card reads `manifest.project.maturity`
  and renders the level verbatim (one of `L0`, `L1`, `L2`,
  `L3`, `L4`); the card also reads
  `manifest.project.target_maturity`,
  `manifest.profile.id` (or `manifest.project.profile` as
  the fallback), `manifest.project.path`, and
  `manifest.project.observed_at`

#### Scenario: Maturity description is operator-gloss
- **WHEN** the project is at maturity `L0`
- **THEN** the lifecycle summary reads "prototype — bring
  it into the registry" (or the equivalent operator-gloss
  for any other level); a `—` maturity renders the
  fallback "Maturity is not yet declared. The buttons below
  drive the project from here to publish, or to retirement."

### Requirement: Per-project action buttons are grouped by lifecycle stage
Forge SHALL group the per-project action buttons the
workbench renders into four lifecycle stages, in this
order: **Adopt**, **Day-to-day**, **Release**, **Retire**.
A stage is shown only if the open project has at least one
executable action in that stage; an empty stage is hidden.
The grouping is derived from each command's catalog id
(`new.*` / `import.*` / `register.*` / `workspace.*` /
`graduation.*` → Adopt; `feature.*` / `spec.*` /
`upgrade.*` / `doctor.*` / `kit.*` / `test.*` / `commit.*`
→ Day-to-day; `release.*` / `deploy.*` / `publish.*` /
`delivery.*` → Release; any future `retire.*` / `deprecate.*`
→ Retire; the default for unmatched ids is Day-to-day).

#### Scenario: Buttons appear in the lifecycle order
- **WHEN** the workbench renders the actions card for a
  project that has executable rows in two or more stages
- **THEN** the buttons are listed top-to-bottom in the
  order Adopt, Day-to-day, Release, Retire; a stage with
  zero executable rows is omitted entirely

#### Scenario: Every button is a confirm-gated control
- **WHEN** the operator clicks any button in any stage
- **THEN** the existing preview → confirm flow runs: a
  preview returns a digest and writes nothing; the
  operator must tick the confirm checkbox and apply the
  exact plan they reviewed; a stale or changed plan is
  refused and nothing is written

#### Scenario: Empty workbench
- **WHEN** the catalog JSON endpoint returns zero executable
  rows for the open project
- **THEN** the actions card renders a single muted line:
  "No browser-executable actions are available for this
  project right now. Reload after the API and its catalog
  are reachable."

### Requirement: Catalog JSON endpoint is the only data source
Forge SHALL continue to expose the catalog at
`GET /v1/admin/commands`. The workbench is the only browser
consumer of that endpoint; the catalog-as-reference page
is gone.

#### Scenario: Workbench fetches the catalog
- **WHEN** the workbench loads
- **THEN** `app.js` issues `GET /v1/admin/commands` exactly
  once and stores the response in `catalogCommands`

#### Scenario: Catalog fetch failure
- **WHEN** the catalog JSON endpoint is unreachable
- **THEN** the actions card renders the empty-workbench
  message and the workbench still loads; no per-page error
  is shown because there is no per-page to show it on
