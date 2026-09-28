# Capability: portfolio-activation-readiness

## ADDED Requirements

### Requirement: Read-only activation readiness verdict

Forge SHALL report, per registered project, whether its persisted
aggregate interest evidence is sufficient to justify the product-owned
activation follow-up, and SHALL carry the window, source, source
revision, privacy mode, coverage and freshness the verdict rests on.

#### Scenario: Project has reviewed exact evidence above the threshold

- **WHEN** a project has a current, non-stale, complete-coverage,
  exact-count window whose metric meets the declared threshold
- **THEN** Forge reports the project as `ready` with the window, source
  and provenance the verdict rests on, and takes no action itself

#### Scenario: No threshold is declared

- **WHEN** readiness is requested without an operator-declared threshold
- **THEN** Forge reports `not-ready` with reason `threshold-not-declared`
  rather than substituting a default commercial judgement

#### Scenario: Threshold is out of range

- **WHEN** the declared threshold is outside the accepted range
- **THEN** Forge refuses with a typed `portfolio-interest-invalid` and
  reports nothing as ready

#### Scenario: A readiness request changes nothing

- **WHEN** readiness is requested for any project
- **THEN** no snapshot, metric, finding, journal row or registry byte is
  written

### Requirement: Absence and doubt never read as readiness

Forge SHALL report `not-ready` with a named reason for every condition
that withholds readiness, and SHALL NOT treat an absent, inexact, stale,
partially measured or superseded observation as sufficient evidence.

#### Scenario: Project has no evidence

- **WHEN** a registered project has no stored snapshot
- **THEN** Forge reports `not-ready` with reason `no-evidence` and
  invents no zero

#### Scenario: Evidence is stale

- **WHEN** every current window is older than the staleness bound
- **THEN** Forge reports `not-ready` with reason `stale-window` and does
  not present the stale figure as current

#### Scenario: Privacy mode is not an exact count

- **WHEN** the evidence carries `lower-bound` or `undeclared` privacy mode
- **THEN** Forge reports `not-ready` with reason `inexact-privacy-mode`
  and never reads the figure as a headcount

#### Scenario: Coverage is partial

- **WHEN** the evidence declares `partial` coverage
- **THEN** Forge reports `not-ready` with reason `partial-coverage` and
  never reads the window as a complete measurement

#### Scenario: Only superseded revisions remain

- **WHEN** every stored snapshot for the project has been superseded
- **THEN** Forge reports `not-ready` with reason `superseded-only`

#### Scenario: Several conditions hold at once

- **WHEN** more than one condition withholds readiness
- **THEN** Forge reports every reason rather than collapsing them into a
  single score or percentage

### Requirement: Forge gates activation without becoming a billing surface

Forge SHALL NOT offer or imply billing, subscription, entitlement,
checkout, CRM or revenue-attribution behaviour, and SHALL NOT treat an
aggregate demand signal as a payment record or as a grant of access.

#### Scenario: A caller looks for a price or an entitlement

- **WHEN** any Forge transport or projection is inspected
- **THEN** no price, plan, subscription, entitlement, checkout or
  revenue-attribution field exists

#### Scenario: Paid-interest signal is observed

- **WHEN** a project's evidence includes `paid_interest_events`
- **THEN** Forge reports it as an aggregate signal only, and it grants no
  access and authorizes no purchase

#### Scenario: Readiness is established

- **WHEN** a project is reported `ready`
- **THEN** Forge names the selected product as the owner of the
  activation package and starts no product, billing or entitlement work
