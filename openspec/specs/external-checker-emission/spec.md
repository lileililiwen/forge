# external-checker-emission Specification

## Purpose
Project Forge's read-only assessment evidence into the Driftwatchdog
external-checker protocol so any monitored project can gate on Forge
findings without coupling its build to Forge.
## Requirements
### Requirement: Checker-compatible document emission

Forge SHALL emit, on stdout and as the only stdout content, a document that
carries an `alerts` array (possibly empty) following the Driftwatchdog
external-checker protocol, with severities limited to the protocol
vocabulary and every field passed through credential redaction.

#### Scenario: Clean project

- **WHEN** a registered project has no fail or warn evidence
- **THEN** `forge check` exits zero and emits `{"alerts": []}` plus metadata
  fields that a protocol-tolerant parser ignores

#### Scenario: Mixed findings

- **WHEN** doctor and governance observations report fail and warn results
- **THEN** each becomes an alert with severity `error` or `warning`, a stable
  `symbol`, and a project-relative or plane-named `source`

#### Scenario: Missing evidence surfaces as warning

- **WHEN** a plane reports `unverified` or `unavailable` for the target
- **THEN** an alert names the plane and the missing evidence instead of
  being silently dropped

### Requirement: Read-only guarantee

A `forge check` run SHALL NOT mutate the assessed project, its manifest,
registry records, `.forge/` state, Git working tree or operation journal.

#### Scenario: No side effects

- **WHEN** `forge check` runs twice on the same project
- **THEN** manifest bytes, registry contents and journal row counts are
  identical before and after, and the emitted document differs only in
  `generated_at`

### Requirement: Bounded failure surface

Forge SHALL bound document size, SHALL exit zero with a valid document even
when findings exist, and SHALL exit non-zero with a typed error on stderr
without writing any document to stdout only for Forge-side operational
failures.

#### Scenario: Oversized findings

- **WHEN** the mapping produces more alerts than the configured maximum
- **THEN** the document truncates to the bound and carries a final summary
  alert naming the dropped count

#### Scenario: Unregistered target

- **WHEN** the target has no Forge registration
- **THEN** stdout stays empty, a typed error exits non-zero, and no partial
  document is emitted

