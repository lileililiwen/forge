# platform-contract-consumption (delta)

## ADDED Requirements

### Requirement: Declared contract version per family

Forge SHALL register, in one reviewed table, the wire version it emits for
every supported platform family, SHALL build each envelope's `contract`
discriminator from that table rather than from a literal, and SHALL derive
`forge contract inspect`'s declared version from the same table, so a
consumer can tell a `0.1.0` document from a `1.0.0` document. The table
SHALL cover exactly the supported families with no duplicate family and
every value SHALL match the platform contract pattern. Validation SHALL
accept a document whose major the family's vendored registry record still
serves, SHALL refuse a major it does not, and SHALL fail closed when the
vendored registry cannot be read or does not record the family.

#### Scenario: Emitted envelope declares the registered version

- **WHEN** `forge contract emit gate-result <target>` runs and
  `platform.gate-result` is registered at `1.0.0`
- **THEN** the envelope's `contract` reads `platform.gate-result/1.0.0`, and
  `forge contract inspect platform.gate-result` reports the same version
  from the same table

#### Scenario: Undeclared major is refused

- **GIVEN** the vendored registry serves majors `[0, 1]` for
  `platform.gate-result`
- **WHEN** a document declaring `platform.gate-result/2.0.0` is validated
- **THEN** validation refuses naming the family and the undeclared major,
  and no schema check is reported as if it had passed

#### Scenario: Registry cannot answer, so validation refuses

- **GIVEN** the vendored registry is unreadable, unparseable, or records no
  entry for the declared family
- **WHEN** a document is validated
- **THEN** validation refuses naming the registry and the family, and never
  treats the missing major list as an accepted version

#### Scenario: Registered version and supported families stay in step

- **WHEN** the registry table and `supported_families()` are compared
- **THEN** they cover the same families, with no duplicate family and no
  family left unregistered

## MODIFIED Requirements

### Requirement: Refusing status mapping

Forge SHALL map its own status vocabularies to contract status values only
through a total mapping that returns an explicit refusal for any value with
no contract equivalent, and SHALL NOT default an unmapped, unverified or
unknown value into a passing or healthy contract status. For
`platform.gate-result` the mapping SHALL project each recorded gate state
onto the value that states exactly the claim Forge observed: a run that
passed onto `passed`, an observed check failure onto `failed`, a proven
inapplicable check onto `skipped`, and a state Forge could not classify onto
`review_required`, because no one has decided that outcome. The mapping
SHALL NOT render `errored` for any gate state, because Forge's gate plane
never observes a check it could not evaluate to a verdict — that condition
is an unavailable run, which never becomes evidence and therefore never
reaches a payload. A blocked aggregate SHALL render `failed` when a check
row recorded a failure and `review_required` when no check row did, and
SHALL NOT render `errored` in either case.

#### Scenario: Mapped verdicts project

- **WHEN** a gate aggregate of passed, a non-blocking failed or a blocked
  record carrying a failed check row is projected
- **THEN** the envelope carries respectively `passed`, `failed` and
  `failed`, and the Forge record itself is unchanged

#### Scenario: Unclassifiable states project as an open decision

- **GIVEN** a gate aggregate Forge could not classify, a check whose
  state string is outside Forge's vocabulary, or a blocked record whose
  check rows attribute no failure
- **WHEN** the record is projected
- **THEN** the root `result` and each unclassifiable check's `status` both
  read `review_required`, the check keeps its raw status in the redacted
  `detail`, and neither renders `errored`, `failed` or `skipped`

#### Scenario: A block Forge never saw a check fail stays undecided

- **GIVEN** a blocked gate record whose check rows are all passing,
  inapplicable, unclassifiable or absent
- **WHEN** the record is projected
- **THEN** the root `result` reads `review_required` and never `failed`,
  because no attempted check reported a failure that Forge could report

#### Scenario: Unverified readiness

- **WHEN** a readiness result is `unverified`
- **THEN** projection is refused naming the family and the value, and no
  `ready` or `not_ready` document is emitted

#### Scenario: Unknown journal state in a batch

- **WHEN** an audit-event projection encounters a journal row whose state has no
  contract equivalent
- **THEN** that row is dropped and named while the remaining rows still project

#### Scenario: Contradiction never upgrades

- **WHEN** a source record both claims a pass and carries a blocking or
  contradictory signal
- **THEN** the projection reports the non-passing interpretation, mirroring
  the gate plane's existing downgrade rule

### Requirement: Read-only contract projection surface

Forge SHALL provide `forge contract list`, `forge contract inspect <family>`,
`forge contract emit <family> [TARGET]` and `forge contract validate <file|->`
in human and JSON form, and `emit` SHALL derive its envelope from an existing
Core record without writing files, journaling operations or mutating state.
Each emitted envelope SHALL declare the registered version of its family.

#### Scenario: Emit gate result from persisted evidence

- **WHEN** a project has fresh revision-bound gate evidence and
  `forge contract emit gate-result <target>` runs
- **THEN** one `platform.gate-result/1.0.0` envelope prints, validates against
  the vendored schema, and carries the same aggregate, revision, runtime name
  and timestamp the evidence record holds

#### Scenario: Emit with no underlying record

- **WHEN** the requested family has no source record for the target
- **THEN** the command reports the missing record and emits no envelope rather
  than synthesizing a document

#### Scenario: Projection reads nothing else

- **WHEN** any `forge contract` command completes against a project tree
- **THEN** the manifest, registry, operations journal, `.forge/**` files and git
  HEAD are byte-identical to their state before the call

#### Scenario: Validate accepts canonical and refuses invalid fixtures

- **WHEN** `forge contract validate` reads a canonical valid fixture and then an
  invalid one from the vendored fixture set
- **THEN** the first exits zero and the second refuses naming the schema
  location of the failure