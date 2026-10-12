# gate-runtime-evidence Specification

## Purpose
Connect Forge to a project's declared shared gate runtime so gate
execution becomes timestamped, revision-bound evidence visible in doctor,
readiness and the provider matrix instead of a manual assertion.
## Requirements
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

### Requirement: Plan rehearsal document consumption

Forge SHALL recognise the gate runtime's machine-readable plan rehearsal
document by the document's own shape — a root contract in the
`driftwatch-gate-plan/` family together with the rehearsal document kind
and a `dry_run: true` marker, never by the runtime's reported version —
SHALL render its resolved plan (planned checks with their required flag,
bound command source and timeout, blocking policy, manifest digest, rule
pack and blocking prediction) instead of the raw document text, and SHALL
never persist a rehearsal, journal it, or derive an aggregate, check
state or journal verdict from it.

#### Scenario: Rehearsal document is rendered as a plan

- **WHEN** `forge gate --dry-run` receives a document declaring the
  rehearsal contract, rehearsal document kind and `dry_run: true` that
  parses in full
- **THEN** the output names every planned check with its required or
  optional flag, its bound command source and its effective timeout,
  plus the blocking policy, manifest digest, rule pack and blocking
  prediction, with no raw document text and no verdict field

#### Scenario: A rehearsal is never evidence

- **WHEN** a rehearsal document is consumed
- **THEN** no evidence record is persisted for the project, no journal row
  is written, no aggregate or check state is derived from it, and the
  journal verdict vocabulary for the rehearsal is a non-outcome word, so
  no surface can read the rehearsal as a passed, blocked or failed run

#### Scenario: Rehearsal document answers a non-dry-run invocation

- **WHEN** a non-dry-run gate invocation receives a rehearsal document
- **THEN** the runtime answer is reported unavailable naming the
  contradiction, and prior persisted evidence stays byte-identical

#### Scenario: Unrecognised rehearsal shape degrades to a plan preview

- **WHEN** the rehearsal markers are absent, incomplete or contradicted —
  including a document claiming execution or persistence, a `would_block`
  value of `false`, a renamed required field, or an unrecognised contract
  family
- **THEN** the bounded text is reported as a plan preview that persists
  nothing, journals nothing and is never a verdict

#### Scenario: The real status document parses unchanged

- **WHEN** the runtime returns its real status document carrying a
  blocking bit and result rows
- **THEN** Forge maps it to evidence exactly as before, never treats it as
  a rehearsal, and a passing status document still persists a
  revision-bound passing record and journals `done`
