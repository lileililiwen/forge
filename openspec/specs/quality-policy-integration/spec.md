# quality-policy-integration Specification

## Purpose
Delegated quality execution through the DriftWatch policy plane: configured checks invoked with bounded waits, findings mapped to project observations with category, policy ID, severity, tool version and evidence, unavailable (never pass) on adapter failure, and one verdict about a report that every consumer of it reaches.
## Requirements
### Requirement: Delegated policy execution

Forge SHALL invoke configured DriftWatch checks and map findings to project observations while retaining category, policy ID, severity, tool version and evidence. Every consumer of a policy report SHALL reach the same verdict about that report: a report whose applicable rule failed is a failure, a report produced against an older source state is stale, a report in which no rule applied is unavailable and never a pass, and a report in which every applicable rule passed is the only report that may be called healthy.

#### Scenario: Delegated policy execution success

- **WHEN** a supported DriftWatch instance returns findings
- **THEN** doctor displays normalized results linked to their original evidence

#### Scenario: Delegated policy execution failure

- **WHEN** DriftWatch is missing, incompatible, times out or emits invalid output
- **THEN** the run reports unavailable or failed without a quality PASS

#### Scenario: Delegated policy execution boundary

- **WHEN** a policy is not applicable to a profile
- **THEN** the reason is preserved rather than manufacturing a detector result

#### Scenario: A second consumer of the same report

- **WHEN** a surface other than the doctor reads the same policy report
- **THEN** it reaches the same verdict as the doctor and never captures
  the report as a healthy pass when the doctor does not call it healthy

### Requirement: Quality result isolation

Forge SHALL scope policy execution to the selected project and redact credentials from captured evidence.

#### Scenario: Quality result isolation success

- **WHEN** two projects run quality checks
- **THEN** each registry observation references only its own execution

#### Scenario: Quality result isolation failure

- **WHEN** an external finding contains a credential-like value
- **THEN** the stored and displayed evidence redacts it

#### Scenario: Quality result isolation boundary

- **WHEN** a previous successful observation is older than the current source state
- **THEN** doctor marks it stale and does not treat it as current verification

### Requirement: Canonical DriftWatch CLI invocation

Forge SHALL invoke DriftWatch through flags the installed binary actually
supports, using the project working directory as the execution scope, and
SHALL map a parseable DriftWatch document — including one that reports a
blocked gate or failing checker — to normalized findings rather than to an
unavailable result.

#### Scenario: Real binary round trip

- **WHEN** a compatible DriftWatchdog binary is installed and a project runs
  the policy plane
- **THEN** Forge invokes only supported subcommands and flags, parses the
  returned machine-readable document and records findings with their
  checker, category, severity and evidence

#### Scenario: Blocked gate is evidence, not adapter failure

- **WHEN** DriftWatch exits non-zero while emitting a parseable status
  document
- **THEN** Forge reports the corresponding fail findings and does not mark
  the policy plane unavailable

#### Scenario: Unsupported document contract

- **WHEN** the returned document names a contract version Forge cannot parse
- **THEN** Forge reports unavailable with the version named and never renders
  PASS

### Requirement: Policy binary resolution

Forge SHALL resolve the policy binary by ordered probe — explicit
environment override first, then the `driftwatchdog` command name, then the
`driftwatch` alias — and SHALL record the resolved binary name and version in
each observation.

#### Scenario: Cargo-installed sibling host

- **WHEN** only `driftwatchdog` is on PATH and no override is set
- **THEN** Forge resolves and runs it successfully

#### Scenario: No binary present

- **WHEN** neither name is on PATH and no override is set
- **THEN** the policy plane reports unavailable and local workflows continue

#### Scenario: Operator override wins

- **WHEN** `FORGE_DRIFTWATCH_BIN` names a fixture or specific binary
- **THEN** Forge runs exactly that binary without further probing

