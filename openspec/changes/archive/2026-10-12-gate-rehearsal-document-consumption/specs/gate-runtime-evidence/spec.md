# gate-runtime-evidence Specification (delta)

## ADDED Requirements

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
