# portal-web-ui (delta)

## ADDED Requirements

### Requirement: Observability shortcuts on flagged rows

Failed doctor/status rows, stale fleet rows and unfinished Hermora
verbs SHALL link to their existing remediation surface with the exact
CLI shown.

#### Scenario: Failed check previews its remediate plan

- **WHEN** a doctor finding or status check renders as failed or
  unavailable
- **THEN** its row carries a `Plan remediate` button rendering the
  full `forge remediate plan --finding <id>` string, copying it via
  the clipboard with a select-the-text fallback, and never sending a
  filesystem path

#### Scenario: Stale rows link to reconcile

- **WHEN** a fleet row is conflicted, stale, unavailable or publish-failed
- **THEN** its action cell appends a `Refresh & reconcile` deep link
  to `/management?project=<id>` while healthy rows keep exactly
  their Open/Manage control

#### Scenario: Unfinished Hermora retries inline

- **WHEN** the delivery projection records a Hermora verb that is not done
- **THEN** the delivery card shows a `Retry Hermora` control opening
  the existing `delivery.hermora-retry` action card beside the exact
  CLI string

### Requirement: Rail keyboard and mobile hardening

The series rail SHALL be keyboard-operable with text alternatives,
and the shell SHALL stay overflow-free at 390px with sticky headers.

#### Scenario: Roving rail with named rows

- **WHEN** the operator tabs to the rail and uses arrows/Home/End
- **THEN** exactly one rail link is in the Tab order, focus moves
  within the rail, and every row announces `<Label>:
  <done|current step|upcoming>` with a single `aria-current="step"`

#### Scenario: 390px shell with sticky context

- **WHEN** the workbench renders at 390 CSS px
- **THEN** the 232px sidebar presents as a topbar row under 860px,
  table headers stick inside their scroll regions, panel tool
  clusters wrap, and the page has no horizontal overflow

### Requirement: Lifecycle rail browser oracle

The change SHALL ship a live Chromium oracle proving rail shape,
coexistence, copy exactness, roving, overflow and contrast.

#### Scenario: Pinned harness verifies the rail end to end

- **WHEN** `tests/lifecycle_rail_browser.rs` runs with the pinned
  `tests/browser` playwright 1.63.0 and Chromium available
- **THEN** `lifecycle-rail-check.mjs` verifies 8 ordered steps, one
  current, one Tab stop, step/project coexistence across
  load/click/reload, bogus-step tolerance, one Next, arrow roving,
  clipboard-equals-shown copy, 390px no overflow and contrast AA;
  without the toolchain it reports UNVERIFIED (exit 2), never a pass

## ADDED Requirements

### Requirement: Lifecycle series rail in the workbench

The workbench lifecycle area SHALL render an ordered series rail with
the 8 steps Idea, Scaffold, Spec, Code, Test, Release, Deploy and
Operate, derived from the existing doctor, status, delivery and
journal projections, with exactly one step carrying
`aria-current="step"` and every step deep-linking with both
`?project=` and `?step=`.

#### Scenario: Eight derived steps with a single current

- **WHEN** a managed project is open in the workbench
- **THEN** `ol#lifecycle-rail` lists the 8 steps in series order,
  each state derived from manifest maturity/target, doctor health,
  status checks, delivery phase/verbs/next and journal operation
  kinds, with the first incomplete step (or Operate when all are
  done) carrying `aria-current="step"` and no step inventing
  evidence its projection did not supply

#### Scenario: Step deep link preserves the project

- **WHEN** the operator follows or reloads a rail step link
- **THEN** the URL carries both `?project=<id>` and `?step=<key>`,
  project switches preserve a valid step, step clicks preserve the
  project, applying the step scrolls to the mapped card without
  moving `aria-current`, and unknown step values are ignored

### Requirement: Next-best-action card in the workbench

The workbench SHALL render a single next-best-action card computed
from maturity, target and evidence, naming the one action, the
reason, and the exact CLI.

#### Scenario: One ranked Next with reason and CLI

- **WHEN** the workbench projections resolve
- **THEN** the next-best-action card shows the single ranked pick
  (declare maturity, run doctor, pick target, unblock delivery,
  mapped delivery action, plan upgrade, or sustain watch) with a
  human reason, the exact CLI when the mapped catalog command is
  known, and a control opening the matching action card

### Requirement: Copy-as-CLI on every confirm action

Every catalog-driven confirm action SHALL carry a Copy-as-CLI button
rendering the exact `forge ...` string built from the catalog
`cli_invocation`, the project positional for scoped rows, and the
card's own typed payload, with secrets redacted.

#### Scenario: Full string shown and copied

- **WHEN** the operator opens any confirm action and activates Copy-as-CLI
- **THEN** the full untruncated `forge ...` string renders in the
  card, copies via the clipboard with a select-the-text fallback,
  and secret-ish fields (`confirm*`, `*token*`, `secret_ref`) appear
  only as `<token>`/`<redacted>` placeholders
