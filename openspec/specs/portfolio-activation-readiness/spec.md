# portfolio-activation-readiness Specification

## Purpose
TBD - created by archiving change portfolio-activation-readiness. Update Purpose after archive.
## Requirements
### Requirement: Read-only activation readiness verdict

Forge SHALL expose a read-only readiness verdict, contract
`forge-portfolio-activation/0.1.0`, that reports for each evaluated
registered project whether its persisted aggregate interest evidence is
sufficient to justify the product-owned activation follow-up, and SHALL
carry the window, source, source revision, privacy mode, coverage and
freshness the verdict rests on.

The verdict SHALL be the closed set `ready` | `not-ready`; there SHALL be
no score, percentage, ranking or "best project" output.

#### Scenario: Project has exact, current, complete evidence above the threshold

- **WHEN** a project has a current, non-stale, `complete` coverage,
  `exact-count` window whose declared metric meets the declared threshold
- **THEN** the project's verdict is `ready` with no reasons, and its
  evidence names the window, source, source revision, privacy mode,
  coverage, freshness and value

#### Scenario: A readiness request changes nothing

- **WHEN** readiness is requested for any project through any transport
- **THEN** no snapshot, metric, finding, journal row or registry byte is
  written, and no schema or migration change is required to read a
  registry written before this capability existed

#### Scenario: The verdict is deterministic

- **WHEN** the same readiness request is repeated against unchanged
  evidence
- **THEN** the verdicts are ordered by project id ascending, the reasons
  for a verdict appear in the fixed declaration order
  (`threshold-not-declared`, `no-evidence`, `superseded-only`,
  `no-current-window`, `stale-window`, `inexact-privacy-mode`,
  `partial-coverage`, `below-threshold`), and the rendered output is
  byte-identical

#### Scenario: The threshold is declared on the operator's terms

- **WHEN** the operator declares a minimum value
- **THEN** the threshold is bounded to `0..=1_000_000_000` and a value
  outside that range is a typed `portfolio-interest-invalid` refusal with
  nothing reported as ready, never a clamp; a threshold of `0` is legal
  and asks only whether exact fresh complete evidence exists

### Requirement: The readiness gate has a machine signal

Forge SHALL let a caller act on the verdict without parsing prose: the
CLI SHALL exit zero when every evaluated project is `ready` and non-zero
when any is `not-ready`, and the JSON API SHALL answer `200` with the
verdict for both outcomes.

#### Scenario: The CLI gate signals not-ready

- **WHEN** `forge portfolio activation readiness` evaluates any project
  as `not-ready`
- **THEN** the full report is printed to stdout, a typed
  `portfolio-activation-not-ready` error naming the count and the
  distinct reasons is written to stderr, and the process exits non-zero

#### Scenario: An input error is not a verdict

- **WHEN** a metric is unlisted, a threshold or staleness bound is out of
  range, a window is malformed or inverted, a source is blank, or the
  evaluation set is empty
- **THEN** Forge refuses with `portfolio-interest-invalid` and prints
  nothing to stdout, because these are input errors rather than verdicts

#### Scenario: The API reports both verdicts successfully

- **WHEN** `GET /v1/interest/readiness` is called with a valid admin
  session for a `ready` and for a `not-ready` project
- **THEN** both answer `200` with `{"interest": {"activation": …}}` and
  neither constructs the CLI gate error

#### Scenario: The route is admin-gated

- **WHEN** the readiness route is called without a live `admin:access`
  session
- **THEN** Forge answers `401 api-unauthorized` and evaluates nothing,
  because which projects are commercially promising is itself private

### Requirement: Absence and doubt never read as readiness

Forge SHALL report `not-ready` with a named reason for every condition
that withholds readiness, and SHALL NOT treat an absent, inexact, stale,
partially measured, superseded or differently-sourced observation as
sufficient evidence. Every held condition SHALL be reported; conditions
SHALL NOT be collapsed into one.

#### Scenario: Project has no evidence

- **WHEN** a registered project has no stored snapshot
- **THEN** the verdict is `not-ready` with reason `no-evidence`, and
  Forge invents no zero

#### Scenario: Only superseded revisions remain

- **WHEN** every stored snapshot for a project has been superseded
- **THEN** the verdict is `not-ready` with reason `superseded-only`

#### Scenario: No current window reports the metric

- **WHEN** current windows exist but none reports the declared metric, or
  none matches a declared window or source
- **THEN** the verdict is `not-ready` with reason `no-current-window` and
  a detail naming the metric and, when declared, the window or source

#### Scenario: The latest window is stale

- **WHEN** the latest window reporting the metric ended further in the
  past than the declared staleness bound
- **THEN** the verdict is `not-ready` with reason `stale-window` and the
  stale figure is never presented as current

#### Scenario: The latest window is inexact

- **WHEN** the latest window reporting the metric carries `lower-bound`
  or `undeclared` privacy mode
- **THEN** the verdict is `not-ready` with reason `inexact-privacy-mode`,
  and the figure is never read as a headcount

#### Scenario: The latest window is partially measured

- **WHEN** the latest window reporting the metric declares `partial`
  coverage
- **THEN** the verdict is `not-ready` with reason `partial-coverage`, and
  the window is never read as a complete measurement

#### Scenario: Readiness is withheld for several reasons at once

- **WHEN** more than one withholding condition holds
- **THEN** every reason is reported in the fixed order rather than being
  collapsed into one reason or a score

#### Scenario: No threshold is declared

- **WHEN** readiness is requested without a declared minimum value
- **THEN** the verdict includes reason `threshold-not-declared` rather
  than Forge substituting a default commercial judgement

#### Scenario: Readiness is never reached by falling back

- **WHEN** the latest window reporting the metric fails a condition and
  an older window would satisfy it
- **THEN** the verdict rests on the latest window and reports its reason,
  and Forge does not fall back to the older window

### Requirement: Forge gates activation without becoming a billing surface

Forge SHALL NOT offer or imply billing, subscription, entitlement,
checkout, CRM or revenue-attribution behaviour, and SHALL NOT treat an
aggregate demand signal as a payment record or as a grant of access.

#### Scenario: A caller looks for a price or an entitlement

- **WHEN** any Forge transport, projection, table or tool list is
  inspected
- **THEN** no price, plan, invoice, customer, charge, subscription,
  entitlement, checkout or revenue-attribution field exists

#### Scenario: Paid-interest signal is observed

- **WHEN** a project's evidence includes `paid_interest_events`
- **THEN** Forge reports it as an aggregate signal only; it is not a
  payment record, it grants no access, and it authorizes no purchase

#### Scenario: Readiness is established

- **WHEN** a project is reported `ready`
- **THEN** Forge names the selected product as the owner of the
  activation package and begins no product, billing, entitlement or
  checkout work

