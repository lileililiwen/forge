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

### Requirement: Web surface for the gate status

Forge SHALL expose the gate evidence envelope to the workbench Health
card as two session-gated admin routes that wrap the existing
`forge::gate` Core path the CLI uses (`forge::gate::run_gate` and
`forge::gate::load_latest_evidence` + `forge::gate::evidence_freshness`),
so a sibling operator can read the current cached gate status and ask
Forge to re-run the gate and journal a fresh evidence record from the
browser. The browser SHALL be a thin caller: the handler MUST NOT
classify a state the runtime did not report, MUST NOT persist or journal
a rehearsal, and MUST NOT fabricate `passed` from a missing or
unparseable runtime answer. The wire envelope SHALL match the same
`GATE_CONTRACT_VERSION` the CLI prints for `forge gate status` and
`forge gate`, and the freshness field SHALL be computed against the
project's current `git rev-parse HEAD`, so the browser, the CLI and the
doctor's `gate-evidence` finding all report the same aggregate, the same
per-check states, the same short revision and the same freshness for
the same project on the same revision.

#### Scenario: Workbench status read matches the CLI

- **WHEN** a workbench session-gated admin GET reads
  `GET /v1/admin/projects/{id}/gate/status` for a managed project that
  has persisted evidence
- **THEN** the response carries the same `gate` object the CLI prints
  for `forge gate status {id}` — same `aggregate`, same `checks[]`
  (states and notes byte-for-byte), same `runtime`, same
  `runtime_version`, same `manifest_digest`, same `rule_pack_version`,
  same `revision` (short form), same `observed_at`, same `freshness`
  (computed against the current HEAD), and the same `note` — with the
  absolute project directory projected out

#### Scenario: Absent evidence is rendered as unverified

- **WHEN** the workbench status GET targets a managed project that has
  never produced a gate evidence record
- **THEN** the response is `200` with `gate: null`, `freshness:
  "absent"`, `aggregate: "unverified"` and a `detail` field carrying the
  same note the CLI prints: "the gate has never run; unverified is not a
  pass"

#### Scenario: Re-run produces and journals an evidence record

- **WHEN** a workbench session-gated admin POST issues
  `POST /v1/admin/projects/{id}/gate/run`
- **THEN** the handler calls `forge::gate::run_gate` with the default
  `GateConfig::from_env()` (honouring `FORGE_GATE_BIN`), persists the
  returned `GateEvidence` atomically via the existing
  `forge::gate::save_evidence` (the same atomic tmp+rename the CLI
  uses), journals a row through the same `registry.record_operation(
  "gate", id, verdict, summary )` call the CLI uses, and returns the
  parsed `GateEvidence` with `freshness` overlaid — the aggregate and
  per-check states are taken from what the runtime actually said

#### Scenario: Runtime unavailable preserves prior evidence

- **WHEN** the re-run POST returns a `GateOutcome::Unavailable` because
  the runtime could not be resolved, timed out, or returned an
  unparseable document
- **THEN** the wire envelope is `502 gate-runtime-unavailable` with the
  bounded reason, the persisted evidence file is left byte-identical,
  no journal row is written, and the workbench sub-row renders the same
  `Unavailable` label the doctor's `gate-evidence` finding already
  renders for the same condition — never `Passed`

#### Scenario: Rehearsal on a real run is refused, not persisted

- **WHEN** the re-run POST returns a `GateOutcome::Rehearsal { .. }`
  because the runtime answered a rehearsal document on a real-run
  invocation
- **THEN** the wire envelope is `409 gate-rehearsal-not-evidence` with
  the bounded reason, the persisted evidence file is left
  byte-identical, no journal row is written, and the workbench sub-row
  surfaces the refusal in the existing `role=status` notice — a
  rehearsal is never evidence, never a journal row, never a `Passed`

#### Scenario: A state the runtime did not report is never synthesized

- **WHEN** the workbench status GET or re-run POST renders a per-check
  row whose raw `status` is outside the known vocabulary
  (`PASS`/`FAIL`/`SKIP`/`NOT_APPLICABLE`)
- **THEN** the wire carries the row's `state` as `unresolved` and the
  row's `note` carries the explanatory prefix the existing
  `GateCheckState::from_raw` mapping emits ("state `X` is not in the
  known vocabulary and never counts as pass"); the wire carries no
  `passed` aggregate unless the runtime produced a parseable
  `PASS` document with a successful exit, and the top-level aggregate
  is whatever `classify_aggregate` mapped (`blocked` for a blocked
  document, `failed` for `FAIL`, `unknown` for `REVIEW_REQUIRED`,
  `PASS`-but-failed-exit, or any unrecognised top-level status) — never
  a fabricated `passed`

