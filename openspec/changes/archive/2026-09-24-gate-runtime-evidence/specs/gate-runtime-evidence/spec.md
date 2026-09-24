# gate-runtime-evidence Specification

## Purpose

Connect Forge to a project's declared shared gate runtime so gate
execution becomes timestamped, revision-bound evidence visible in doctor,
readiness and the provider matrix instead of a manual assertion.

## ADDED Requirements

### Requirement: Declared gate runtime execution

Forge SHALL resolve and execute the gate runtime a project declares,
through bounded argument-array invocation, and SHALL classify a parseable
status document — including one reporting a blocked gate with non-zero
exit — as evidence rather than adapter failure.

#### Scenario: Passing gate

- **WHEN** the runtime reports aggregate passed for the current revision
- **THEN** fresh evidence is persisted, the journal row verdict is done,
  and `forge gate` exits zero

#### Scenario: Blocked gate

- **WHEN** the runtime exits non-zero with a parseable blocked document
- **THEN** Forge records the blocked aggregate with per-check rows, exits
  non-zero mirroring the block, and reports no unavailable classification

#### Scenario: No runtime resolvable

- **WHEN** the project declares no gate runtime and none is installed
- **THEN** the command reports unavailable listing resolution attempts and
  prior evidence remains untouched

### Requirement: Revision-bound evidence lifecycle

Forge SHALL bind each gate evidence record to the captured source revision
and SHALL treat evidence as stale, never as current verification, once the
working tree revision has moved.

#### Scenario: Stale evidence

- **WHEN** persisted evidence names a revision different from the current
  HEAD
- **THEN** doctor's gate finding warns as stale and readiness claims cannot
  cite it

#### Scenario: Never-run gate

- **WHEN** a project has no persisted gate evidence
- **THEN** the gate plane reports `unverified` and no surface renders that
  as passing

### Requirement: Honest surface projection

The gate evidence SHALL surface through doctor findings,
`forge gate status` and the provider matrix using the shared vocabulary,
with credential redaction applied, and SHALL NOT be exposed as an MCP or
API write surface by this capability alone.

#### Scenario: Read surfaces agree

- **WHEN** a gate run completes and its evidence is read through CLI JSON,
  human output and doctor
- **THEN** every surface reports the same aggregate, revision, runtime name
  and timestamp

#### Scenario: Registry stays silent on gate tools

- **WHEN** `tools/list` is served after gate runs exist
- **THEN** no gate tool appears in the mature MCP registry
