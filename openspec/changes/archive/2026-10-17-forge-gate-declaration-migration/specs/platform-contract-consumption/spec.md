# platform-contract-consumption Specification (delta)

## MODIFIED Requirements

### Requirement: Refusing status mapping

Forge SHALL map its own status vocabularies to contract values only through total mappings that return an explicit refusal for any value with no contract equivalent and SHALL NOT default an unmapped, unverified, unknown, or unbound value to passing or healthy.

For `platform.gate-result/1.0.0`, `map_gate_check_state` SHALL map every `GateCheckState` exactly as follows: `Pass` → `passed`; `Fail` → `failed`; `ReviewRequired` → `review_required`; `Unresolved` → `review_required`; `Unbound` → `unbound`; `NotApplicable` → `skipped`; compatibility-only legacy `Skip` → `skipped`. No Gate row SHALL map to `errored`, because an unavailable run never becomes evidence.

`map_gate_result` SHALL map aggregate state with this precedence: `Passed` → `passed`; `Failed` → `failed`; `Unknown` → `review_required`; `Blocked` → `failed` when any recorded row is `Fail`, otherwise `review_required`. An `Unbound` row SHALL remain `unbound` at row level and SHALL NOT by itself invent either a failed or review-required root result; the authoritative blocked aggregate selects the root branch and the presence or absence of an actual `Fail` row selects only between `failed` and `review_required`.

#### Scenario: Mapped verdicts project

- **WHEN** records with aggregates `Passed`, `Failed`, and `Unknown`, plus blocked records with and without a `Fail` row, are projected
- **THEN** root `result` is respectively `passed`, `failed`, `review_required`, `failed`, and `review_required`, while each Forge record remains unchanged

#### Scenario: Every Gate row state has one contract value

- **WHEN** rows in states `Pass`, `Fail`, `ReviewRequired`, `Unresolved`, `Unbound`, `NotApplicable`, and legacy `Skip` are projected
- **THEN** their statuses are respectively `passed`, `failed`, `review_required`, `review_required`, `unbound`, `skipped`, and `skipped`, with no default branch that can turn a future state into passing

#### Scenario: Unclassifiable states project as an open decision

- **GIVEN** a gate aggregate Forge could not classify or a check whose current raw state is outside Forge's vocabulary
- **WHEN** the record is projected
- **THEN** the root result or row status is `review_required`, the row keeps its raw status in redacted detail, and neither renders `errored`, `failed`, `skipped`, or `passed`

#### Scenario: A block Forge never saw a check fail stays undecided

- **GIVEN** a blocked Gate record whose rows are passing, not-applicable, review-required, unresolved, unbound, legacy-skipped, or absent, and none is `Fail`
- **WHEN** the record is projected
- **THEN** the root result is `review_required`, each `Unbound` row remains `unbound`, and the root never becomes `failed` merely because a row is unbound

#### Scenario: Unverified readiness

- **WHEN** a readiness result is `unverified`
- **THEN** projection is refused naming the family and value, and no `ready` or `not_ready` document is emitted

#### Scenario: Unknown journal state in a batch

- **WHEN** an audit-event projection encounters a journal row whose state has no contract equivalent
- **THEN** that row is dropped and named while remaining rows still project

#### Scenario: Contradiction never upgrades

- **WHEN** a source record both claims a pass and carries a blocking or contradictory signal
- **THEN** the projection reports the non-passing interpretation and never upgrades it to passed

### Requirement: Additive-only compatibility boundary

Forge SHALL preserve every pre-existing field, discriminator, value meaning, exit code, journal row, and generated-tree byte when this capability or another owning capability adds optional fields to an existing tolerant document. Owning capabilities MAY add fields to Gate status, Gate evidence, checker, API, or other tolerant envelopes, but SHALL NOT remove a field, change a pre-existing field's serialized value or meaning, repurpose a discriminator, or require an old consumer to understand the addition. Consumers that accepted the prior document and ignore unknown fields SHALL remain valid. This platform-contract capability itself SHALL remain a CLI contract surface and SHALL add no MCP tool, API route, or portal section.

#### Scenario: Existing surfaces unchanged

- **WHEN** Gate status JSON or the checker document is captured before and after its owning capability adds optional runtime, attribution, freshness, or projection fields
- **THEN** every pre-existing field and value retains the same serialization and meaning, only additive fields may differ, no old field is removed or reinterpreted, and an old tolerant consumer still accepts the document

#### Scenario: Unaffected surfaces remain byte-identical

- **WHEN** the fleet listing, provider matrix, operation journal, generated trees, and unrelated contract envelopes are captured before and after the additive owning-capability change
- **THEN** their bytes are identical unless that same owning capability explicitly owns an additive field on that exact tolerant document

#### Scenario: Dependency graph of the shipped binary

- **WHEN** the workspace dependency manifest is inspected
- **THEN** no schema-validation dependency appears in the runtime dependency set, only in the development set

#### Scenario: Transports advertise nothing new

- **WHEN** the mature MCP tool registry, API route table, and portal section set are listed
- **THEN** no new platform-contract operation appears in any of them; additive fields on already-owned Gate routes are not new contract operations
