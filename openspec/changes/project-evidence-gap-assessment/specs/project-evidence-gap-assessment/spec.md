# Capability: project-evidence-gap-assessment

## ADDED Requirements

### Requirement: Evidence-backed project findings

Forge SHALL produce, contract `forge-project-evidence/0.1.0`, a stable,
evidence-backed finding for each metadata gap in a single project or the whole
catalog, and SHALL carry the finding's category, verdict, remediation class and
the source provenance it rests on.

#### Scenario: A missing description is one finding

- **WHEN** a project has no description, domain or portfolio tags
- **THEN** Forge emits a distinct finding per gap with a stable id, rather than
  collapsing them into a single health value

#### Scenario: A finding names its evidence

- **WHEN** a finding is produced
- **THEN** it carries the source, source revision and observation time, and any
  credential-shaped value is redacted

### Requirement: Verdicts stay distinct

Forge SHALL preserve the closed verdict vocabulary `PASS`, `WARN`, `FAIL`,
`UNAVAILABLE` and `NOT_APPLICABLE`, and SHALL NOT report an unavailable source,
a stale observation or a not-applicable control as healthy or as a failure.

#### Scenario: An unreachable source is unavailable, not failed

- **WHEN** a metadata source cannot be read
- **THEN** the finding is `UNAVAILABLE` and names the source, and the project is
  never reported healthy by silence

#### Scenario: An inapplicable control is not a failure

- **WHEN** a metadata control does not apply to the project's profile
- **THEN** the finding is `NOT_APPLICABLE` and is excluded from any healthy or
  failed count

#### Scenario: A stale observation is not current evidence

- **WHEN** a metadata observation is older than its freshness bound
- **THEN** the finding is reported with its stale state and never presented as
  current

### Requirement: Assessment is read-only

Forge SHALL assess and report without writing any file, registry row, provider
value or CI execution.

#### Scenario: A gap assessment changes nothing

- **WHEN** a gap assessment runs through any transport
- **THEN** no file, registry byte, table row or journal row is written and no
  provider is mutated
