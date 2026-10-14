# control-plane-portal Specification

## Purpose
Read-only control-plane views over the same Core contracts as CLI and MCP: the twelve §36 sections rendered as project or fleet dashboards and single-section views with worst-status rollup, journaling portal reads without mutating any project.
## Requirements
### Requirement: Shared project and lifecycle views

Forge SHALL provide the planned portal sections using shared API resources, including project identity, maturity, quality, agent, deployment and documentation status.

#### Scenario: Shared project and lifecycle views success

- **WHEN** an authorized user opens a project
- **THEN** the portal displays API-backed current observations and navigable related resources

#### Scenario: Shared project and lifecycle views failure

- **WHEN** the API rejects access or is unavailable
- **THEN** the portal displays the actual access or availability error without cached success claims

#### Scenario: Shared project and lifecycle views boundary

- **WHEN** a status observation is stale or unknown
- **THEN** the view labels that state and retains its observation timestamp

### Requirement: Accessible controlled operations

Forge SHALL provide keyboard-accessible responsive operation flows with loading, empty, error, success and partial-result states, preserving Core review and authorization requirements.

#### Scenario: Accessible controlled operations success

- **WHEN** a user starts an authorized lifecycle operation
- **THEN** the UI displays its plan where required and tracks its operation ID to completion

#### Scenario: Accessible controlled operations failure

- **WHEN** an operation fails after a partial external write
- **THEN** the UI shows completed and failed stages plus recovery guidance

#### Scenario: Accessible controlled operations boundary

- **WHEN** the portal is absent or closed
- **THEN** the same supported operations remain available through CLI and mature MCP tools

### Requirement: Forge-administrator password rotation in the topbar

The control-plane portal SHALL expose a `Change password` control
in the dashboard topbar that posts the operator's new password to
`POST /v1/admin/identity/change-password` and, on success,
auto-signs-out so the operator signs back in with the rotated
credential. The control SHALL be visible only when the operator has
a valid `forge_admin_session` cookie; the underlying Core call
SHALL refuse to update a registry with no `forge_admin` row.

#### Scenario: Operator rotates the Forge administrator password

- **WHEN** the operator clicks `Change password` and submits a
  matching current/new/confirm trio
- **THEN** the control posts to the named route, the response is
  `200 { changed: true, contract: "forge-admin-login/1.0.0" }`,
  and the dashboard returns the operator to the sign-in page so
  they re-authenticate with the new password.

#### Scenario: Rotation refusal preserves the existing credential

- **WHEN** the operator submits a weak new password, a
  mismatched confirmation, or a wrong current password
- **THEN** the route returns a typed refusal (`400
  admin-password-too-weak` / `400
  admin-password-confirmation-mismatch` / `403
  admin-current-password-incorrect`), the stored password hash
  is unchanged, and every pre-rotation session is still valid.

#### Scenario: Unconfigured registry refuses to initialize

- **WHEN** the operator submits against a registry with no
  `forge_admin` row
- **THEN** the route returns `403
  admin-identity-not-configured` and never inserts a row.

### Requirement: Topbar workspace status summary

Forge SHALL provide, in the portal topbar of the "All projects" view, a read-only
summary that mirrors the CLI workspace sibling overview: the count of sibling
directories **registered** in the registry, the count **onboardable** (not yet
registered), and a **last-sync** freshness label. The summary SHALL aggregate
from the existing session-gated `GET /v1/admin/workspace/candidates` route and
SHALL introduce no new API route, no write control, and no Core capability.

#### Scenario: Signed-in operator sees workspace counters

- **WHEN** an authenticated admin with a configured `FORGE_ADMIN_PROJECTS_ROOT` opens "All projects"
- **THEN** the topbar shows the registered count (siblings whose candidate `state` is `registered`) and the onboardable count (candidates whose `selectable` is true) aggregated from one candidate request, and a last-sync label derived from the client observation time

#### Scenario: Workspace root is unconfigured

- **WHEN** `FORGE_ADMIN_PROJECTS_ROOT` is unset, blank or not a directory
- **THEN** the candidate route refuses with typed `409 admin-prerequisite` and the topbar shows an honest "Workspace not configured" chip instead of any count — never a fabricated number

#### Scenario: Candidate read fails for another reason

- **WHEN** the candidate request fails with a non-409 transport or auth error
- **THEN** the counters are hidden and a status note reports the workspace summary is unavailable, without claiming a count

#### Scenario: Last-sync becomes stale

- **WHEN** the summary has not been re-read within the bounded idle threshold
- **THEN** the last-sync label switches to a `stale` state and retains its last observation time

#### Scenario: No write control in the summary

- **WHEN** the operator views the topbar summary
- **THEN** no onboard, link or mutation control is present — the summary is a read-only mirror of the candidate route

### Requirement: Responsive layout and reduced-motion rendering

The control-plane SPA portal SHALL render usable at viewports down to
320 CSS px and SHALL honor the `prefers-reduced-motion` user setting.
At `width <= 720px` the portal SHALL collapse the workbench multi-card
grid to a single column and SHALL reflow the 8-column Bulk Workspace
onboarding table (`#ws-table-wrap` / `#ws-rows`) into one stacked card
per row, preserving every one of its eight columns (Onboard, Directory,
Project id, Action, Profile signal, State, Id override, Profile
override) with a readable per-column label, without removing any column
or introducing a horizontal-scroll-only escape. Every interactive
control SHALL remain keyboard-reachable and SHALL keep a `>=44px` touch
target. Under `prefers-reduced-motion: reduce` the portal SHALL disable
transitions and animations on the lifecycle rail, panels, cards, and
the shared button/nav/action controls without hiding or disabling any
control.

#### Scenario: Workbench grid collapses to one column on a phone

- **WHEN** the viewport width is `<= 720px`
- **THEN** `.workbench-grid` lays out as a single column so no card is
  squished below a readable width, and the sidebar remains a reachable
  top row with all nav destinations in the tab order.

#### Scenario: Bulk onboarding table reflows to stacked cards

- **WHEN** the viewport width is `<= 720px` and the operator opens the
  Bulk Workspace onboarding table
- **THEN** each candidate renders as a stacked card where every one of
  the eight columns is shown with a visible label (via `td::before`
  `:nth-child` rules, no `data-label` required), the Onboard checkbox
  and the Id/Profile override text inputs stay focusable and `>=44px`,
  and the table region keeps its `role="region"` and
  `aria-label="Workspace candidates"` with the column headers still
  exposed to assistive technology.

#### Scenario: Reduced motion suppresses transitions without losing function

- **WHEN** the OS requests `prefers-reduced-motion: reduce`
- **THEN** transitions and animations on the lifecycle rail, panels,
  cards, and the button/nav/action controls are disabled, and every
  control, card, and the rail remain fully present and operable.

#### Scenario: Touch targets stay reachable on narrow screens

- **WHEN** the viewport width is `<= 720px`
- **THEN** all controls retain keyboard reach and a `>=44px` minimum
  touch target, with no page-level horizontal scroll introduced by the
  new breakpoint.

