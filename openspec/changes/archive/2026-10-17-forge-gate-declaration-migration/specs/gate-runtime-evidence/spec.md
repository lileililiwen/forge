# gate-runtime-evidence Specification (delta)

## MODIFIED Requirements

### Requirement: Web surface for the gate status

Forge SHALL expose the shared Gate status projection to the workbench Health card through the existing session-gated admin status/run routes. Both routes SHALL enforce declaration preflight before evidence access or runtime spawn: the status route SHALL call `forge::gate::read_gate_status`, which preflights internally before loading evidence, while the run route SHALL call `forge::gate::preflight_gate_declarations` explicitly and pass the returned private-field `GateDeclarationContext` to `forge::gate::run_gate`. The browser SHALL remain a thin caller and SHALL NOT classify a state, blocking decision, aggregate, categorized list, or exit that the runtime did not report.

The current Driftwatchdog row vocabulary SHALL be exactly `PASS`, `FAIL`, `REVIEW_REQUIRED`, `NOT_APPLICABLE`, and `UNBOUND`. A current runtime row outside that set, including `SKIP`, SHALL become `unresolved` with its raw word retained in the bounded note. Serialized pre-change Forge evidence containing row state `skip` SHALL remain decode-compatible, but no current runtime parse SHALL produce it. The projection SHALL retain the runtime's raw top-level status, blocked bit, exact numeric exit code, categorized `failures`, `pending_reviews`, `not_applicable`, and `unbound` lists, and each row's source/severity. `GateAggregate` SHALL remain only a normalized compatibility projection; a real run's command exit SHALL mirror the captured runtime exit.

The status and run routes SHALL return the same `GATE_CONTRACT_VERSION` and `GateStatusProjection` used by CLI, doctor, checker, release preflight, and frontend. Declaration errors SHALL be returned before spawn/read short-circuit: malformed Forge verification is `manifest-invalid`; malformed `.project.json` or the selected Gate TOML/YAML is `gate-invalid`; malformed governance configuration is `governance-invalid` only when resolving a governance-root floor row. An invalid declaration SHALL leave the runtime invocation marker, prior evidence, operation journal, and sibling runtime history byte-identical.

#### Scenario: Workbench status read matches the CLI

- **WHEN** a workbench session-gated admin GET reads `GET /v1/admin/projects/{id}/gate/status` for a managed project with valid declarations and persisted evidence
- **THEN** the response carries the same shared Gate projection as `forge gate status {id}`: the same aggregate, raw runtime status/blocked/exit and categorized lists, checks with state/source/severity/note and implementation source/path, runtime/version, manifest/rule-pack identity, revision, timestamp, freshness reasons and pack expectation, with absolute project paths projected out

#### Scenario: Absent evidence is rendered as unverified

- **WHEN** the workbench status GET targets a managed project whose current declarations are valid and that has never produced Gate evidence
- **THEN** the response is `200` with `gate: null`, `freshness: "absent"`, `aggregate: "unverified"`, and detail `the gate has never run; unverified is not a pass`

#### Scenario: Malformed declaration wins over absent evidence

- **WHEN** the target has no Gate evidence but its present Forge verification, `.project.json`, selected Gate manifest, or applicable governance selection is malformed
- **THEN** declaration preflight returns the corresponding typed error before evidence absence is projected, and no process, evidence write, journal write, or sibling-history write occurs

#### Scenario: Re-run produces and journals an evidence record

- **WHEN** a workbench session-gated admin POST issues `POST /v1/admin/projects/{id}/gate/run` with valid current declarations
- **THEN** the handler passes the preflight context to `forge::gate::run_gate` with `GateConfig::from_env()`, persists the returned real-run evidence atomically, journals it through the existing registry operation, and returns the shared projection; the run response and command exit repeat the runtime's raw result and exact exit rather than reconstructing them

#### Scenario: Invalid declaration prevents a re-run

- **WHEN** a duplicate, null, malformed, unsafe, or symlink-escaping declaration is present before the run POST
- **THEN** the route returns its typed declaration error without invoking Driftwatchdog, and the invocation marker, prior evidence bytes, operation journal, and sibling runtime history remain unchanged

#### Scenario: Runtime unavailable preserves prior evidence

- **WHEN** a preflight-valid re-run returns `GateOutcome::Unavailable` because the runtime cannot be resolved, times out, exits without a numeric code, or returns an unparseable document
- **THEN** the wire reports the bounded runtime-unavailable error, prior evidence remains byte-identical, no journal row is written, and the workbench renders unavailable rather than passed

#### Scenario: Rehearsal on a real run is refused, not persisted

- **WHEN** a preflight-valid real run returns `GateOutcome::Rehearsal` because the runtime answered with a rehearsal document
- **THEN** the route returns `409 gate-rehearsal-not-evidence`, prior evidence remains byte-identical, no journal row is written, and the existing status notice reports the refusal

#### Scenario: Current runtime vocabulary is mapped without synthesis

- **WHEN** status/run projection receives current rows containing `PASS`, `FAIL`, `REVIEW_REQUIRED`, `NOT_APPLICABLE`, `UNBOUND`, and one other raw word
- **THEN** the five current words project respectively as `pass`, `fail`, `review_required`, `not_applicable`, and `unbound`; the other row projects `unresolved` with its raw word in the bounded note; none is merged with another or rendered as legacy `skip`; and raw top-level runtime facts and exit remain unchanged

#### Scenario: A state the runtime did not report is never synthesized

- **WHEN** the shared Gate projection renders a per-check row whose raw status
  is outside the current vocabulary (`PASS`/`FAIL`/`REVIEW_REQUIRED`/`NOT_APPLICABLE`/`UNBOUND`)
- **THEN** the wire carries the row's `state` as `unresolved` and the row's
  note carries the explanatory prefix ("state `X` is not in the known vocabulary
  and never counts as pass"); the response carries no `passed` aggregate unless
  the runtime produced a parseable `PASS` document with a successful exit; the
  root result follows the total platform mapping (`review_required` for
  `Unknown` aggregate, `review_required` when blocked without an actual `Fail`
  row) — never a fabricated `passed`.
