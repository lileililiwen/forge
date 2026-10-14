# gate-runtime-evidence Specification (delta)

## ADDED Requirements

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
