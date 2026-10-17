## MODIFIED Requirements

### Requirement: Exported gate evidence is consumed as untrusted input

Forge SHALL parse a produced gate evidence export through bounded, single
document, vocabulary-gated and revision-gated handling. An unknown field name,
out-of-vocabulary state, unsupported contract major, oversized document or
unparseable document SHALL produce an explicit refusal or honest unavailable
outcome that leaves any previously persisted record untouched. The optional
DriftWatchdog producer resolver used by `forge gate evidence` SHALL remain
separate from `forge gate <TARGET>`, which always executes Forge's bundled
Gate engine.

#### Scenario: Well-formed export consumed

- **WHEN** an export naming only recognized fields with recognized states
  arrives from an attributed producer at the captured revision
- **THEN** the record persists revision-bound with producer attribution and
  the evidence view reports its per-field states

#### Scenario: Unknown field name

- **WHEN** the document carries a field outside the consumed evidence vocabulary
- **THEN** that entry is refused naming the field, the refusal is counted, the
  remaining recognized entries stay intact, and no value is silently accepted

#### Scenario: Capability-only state used as evidence state

- **WHEN** a field carries a state the evidence vocabulary does not permit
- **THEN** the entry is refused as vocabulary misuse rather than normalised into a
  permitted state

#### Scenario: Unparseable or oversized document

- **WHEN** the producer's output cannot be parsed or exceeds the read bound
- **THEN** the surface reports unavailable quoting the producer's own bounded
  words, exits non-zero, and the prior record stays byte-identical

#### Scenario: Explicit producer override is scoped to export

- **WHEN** `FORGE_GATE_BIN` or `FORGE_DRIFTWATCH_BIN` names an executable and
  `forge gate evidence` requests release evidence
- **THEN** Forge invokes that producer only for `gate evidence-export`, while
  `forge gate <TARGET>` continues using the bundled Forge engine regardless
  of the same override

#### Scenario: Export producer resolves from PATH

- **WHEN** no explicit export override is set and a supported DriftWatchdog
  binary is available on `PATH`
- **THEN** `forge gate evidence` resolves the first supported producer and
  consumes its bounded export document
