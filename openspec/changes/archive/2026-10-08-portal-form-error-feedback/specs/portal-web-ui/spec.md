# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Scoped form fields carry visible labels and helper text

Fleet filter inputs, delivery action inputs, and portfolio metadata
inputs SHALL each carry a visible label that persists while typing
(placeholders stay as examples only), plus helper text where the field
needs explanation. Relabelled controls SHALL keep the slice-2 44px
targets and overall layout. The browser SHALL still send only validated
single-segment names and typed fields to the existing endpoints; no new
endpoint SHALL be called.

#### Scenario: Read a fleet filter with its guidance visible

- **WHEN** the operator views the fleet filter row
- **THEN** each of the six inputs shows a persistent visible label and
  the row shows helper text explaining blank-means-any predicate
  matching

#### Scenario: Fill a delivery or portfolio field

- **WHEN** the operator fills a delivery allowlist/publish/reconcile/
  lookup field or a portfolio tag/review field
- **THEN** the field shows a visible label and helper text naming the
  accepted value, and the submitted payload shape is unchanged

### Requirement: Failed submissions show a focusable error summary with per-field links

Every failed submission on a scoped form (login, portfolio, delivery,
workbench action, workspace/management onboarding) SHALL render a
focusable error summary (heading plus one link per failing field),
move focus to it, retain the inline field errors, and wire each inline
error to its field with `aria-describedby` (plus `aria-invalid` while
the error shows). The fleet filter error SHALL use `role="alert"`.

#### Scenario: Submit with a missing value

- **WHEN** the operator submits a scoped form with a missing or refused
  value
- **THEN** focus moves to the summary, each summary link targets its
  field, each field keeps its inline error, and the error id is present
  in the field's `aria-describedby` until cleared

#### Scenario: Catalog predicate read fails

- **WHEN** the fleet filter predicate read fails
- **THEN** the row-level error is exposed as `role="alert"` while the
  full table keeps showing

### Requirement: Login offers a password toggle and visible required indicators

The login form SHALL offer a show/hide password toggle (native button,
`aria-pressed`, labelled Show/Hide) and SHALL mark required fields with
a visible indicator plus a legend. `name`, `autocomplete`, and paste
behavior SHALL stay intact so password managers keep working.

#### Scenario: Reveal the typed password

- **WHEN** the operator activates the toggle
- **THEN** the password field switches between hidden and readable, the
  toggle label and `aria-pressed` follow, and the typed value is
  preserved

#### Scenario: Sign in with a password manager

- **WHEN** the operator fills login via a password manager or paste
- **THEN** the fields accept the fill (unchanged `name`/`autocomplete`,
  no paste blocking) and failed sign-in still reports the generic error
  through the summary pattern
