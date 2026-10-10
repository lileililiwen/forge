# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Dashboard exposes a read-only command-reference browser

The dashboard's projects view SHALL carry a read-only command
reference section listing every `GET /v1/admin/commands` catalog row
from the already-fetched catalog JSON, with no new endpoint, no new
fetch and no new frontend dependency. Every row SHALL show its
availability badge, summary and exact `forge ...` CLI string; every
non-web row (`cli_only`, `not_yet_web`, `provider_required`,
`project_capability_required`, `disabled`) SHALL additionally show its
plain-language reason and a clipboard Copy button for the CLI string,
and SHALL never render an executable control. Every `web` row SHALL
link to the existing view that already serves it (`/projects`,
`/workbench`, `/management`, `/portfolio` or `/delivery`).

#### Scenario: Operator discovers what the browser cannot do

- **WHEN** the operator opens the projects view with the catalog loaded
- **THEN** the reference lists every catalog row, each non-web row
  shows its availability badge, plain-language reason and exact CLI
  string with a Copy button, and no non-web row offers an executable
  control

#### Scenario: Operator follows a web row to its view

- **WHEN** the operator activates a web row's link
- **THEN** the browser navigates to the existing view serving that
  command through the standard router (deep-link safe)

#### Scenario: Catalog unavailable

- **WHEN** the catalog fetch fails or returns no rows
- **THEN** the reference renders an honest unavailable state instead
  of an empty list or a stale copy

### Requirement: Command reference is searchable, filterable and accessible

The reference SHALL offer a text search (matching id, label, summary,
CLI string, reason and category) and an availability filter covering
all six catalog states, combined as AND, with a live result count and
an honest no-matches state. Controls SHALL be native keyboard-operable
elements with visible focus; the count and copy feedback SHALL be
`role="status"` live regions; essential text SHALL meet the 12px floor;
no new motion SHALL be introduced; all catalog strings SHALL be
rendered as text (never interpreted as HTML); and no shell, path or
command submission surface SHALL be added.

#### Scenario: Operator narrows the reference

- **WHEN** the operator types in the search box or picks an
  availability state
- **THEN** only matching rows render and the live count announces the
  narrowed result

#### Scenario: Screen-reader operator copies a CLI string

- **WHEN** the operator activates a row's Copy button
- **THEN** the exact CLI string is written to the clipboard (or a
  select-the-text fallback is offered) and the copy live region
  announces the outcome
