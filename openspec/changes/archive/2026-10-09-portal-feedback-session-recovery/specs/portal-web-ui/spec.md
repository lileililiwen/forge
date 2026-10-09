# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Portal action results are announced as live regions

Every portal action result region (workbench plan/apply, delivery
action and lookup, workspace-bulk preview/run, scoped-management
preview/run, and each catalog-action card's preview/refusal/success)
SHALL be a live region. A successful, preview, or informational result
SHALL announce politely (`role="status"`); a refusal SHALL announce
assertively (`role="alert"`). The role SHALL be set on every write so a
region reused for a later outcome never keeps the previous urgency, and
the existing visible result markup SHALL be retained.

#### Scenario: Screen reader hears a completed action

- **WHEN** a screen-reader operator runs a confirmed portal action and
  the operation succeeds
- **THEN** the result region is a polite live region and its success text
  is announced

#### Scenario: Screen reader hears a refusal

- **WHEN** a portal action, preview or onboarding run is refused
- **THEN** its result region is an assertive live region and the refusal
  text is announced, alongside the existing focusable error summary

#### Scenario: A reused result region does not keep stale urgency

- **WHEN** a result region previously showed a refusal and is then
  reused for a successful action
- **THEN** the region announces politely for the success

### Requirement: An expired mid-session credential returns to sign-in

When a protected Forge admin request made from a dashboard page returns
`HTTP 401`, the frontend SHALL return the operator to the sign-in page
with the current same-origin path and query as the `next` deep link,
instead of showing a generic failure with no recovery. The login page
SHALL NOT trigger this redirect, so a rejected credential on the login
page still renders its inline error and summary.

#### Scenario: Session expires during dashboard use

- **WHEN** the operator's Forge-wide session expires or is revoked while
  the dashboard is open and the next admin request returns `401`
- **THEN** the browser navigates to `login.html?next=<current path and
  query>` and no dashboard action control is left claiming success

#### Scenario: Wrong password on the login page

- **WHEN** the operator submits incorrect credentials on the login page
  and the API returns `401`
- **THEN** no redirect occurs and the inline field error and error
  summary render as before

#### Scenario: Return path stays same-origin

- **WHEN** the session-expiry redirect is followed after sign-in
- **THEN** the `next` value is validated as a same-origin dashboard route
  before navigation and an off-origin or malformed value is dropped

### Requirement: Essential portal text meets a 12px floor

Portal text that carries content an operator must read — error text,
helper/hint text, evidence chips, digests, action CLI hints, findings,
workflow reasons, source metadata and detail rows — SHALL render at no
less than 12 CSS pixels. Uppercase micro-labels and badges MAY remain
smaller. The change SHALL NOT reduce contrast or alter the declared base
size.

#### Scenario: Read an error or digest

- **WHEN** the operator reads a field error, a validation hint, an
  evidence chip, a digest or a workbench action's CLI hint
- **THEN** the text renders at 12px or larger

#### Scenario: Prior sizes and tokens survive

- **WHEN** the shipped stylesheet is inspected after this change
- **THEN** the base `16px` declaration, the WCAG contrast pairs and the
  existing `--text-*` tokens are unchanged, and the 44px touch minima are
  intact

### Requirement: Workbench disclosure controls name the panel they control

Each workbench catalog-action disclosure button SHALL reference the
panel it expands with `aria-controls`, in addition to its existing
`aria-expanded` state.

#### Scenario: Assistive technology inspects a disclosure button

- **WHEN** assistive technology or an accessibility audit inspects a
  workbench action's disclosure button
- **THEN** the button carries `aria-expanded` and an `aria-controls`
  value that matches the id of the `.wb-action-body` it discloses
