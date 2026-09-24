# quality-policy-integration Specification

## ADDED Requirements

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
