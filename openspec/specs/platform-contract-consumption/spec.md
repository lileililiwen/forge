# platform-contract-consumption Specification

## Purpose

Forge vendors the published contract schemas under `contracts/` behind a digest-manifest, maintains a versioned surface inventory, provides read-only `forge contract` projection commands, and maps status vocabularies to contract values only through a total mapping that refuses unmapped values instead of defaulting them to healthy.
## Requirements
### Requirement: Versioned surface inventory

Forge SHALL maintain one inventory table of every versioned contract surface it
produces, recording module, constant, discriminator, version, the platform
family it projects to if any, and its documentation path, and SHALL fail its own
tests when a versioned constant exists outside that inventory or when an
inventory row disagrees with the live constant.

#### Scenario: New versioned surface registered

- **WHEN** a module declares a versioned contract constant and adds the matching
  inventory row
- **THEN** the completeness and agreement tests pass and `forge contract list`
  renders the row

#### Scenario: Unregistered versioned surface

- **WHEN** a module declares a `"0.1.0"`-style version constant with no
  inventory row
- **THEN** the completeness test fails naming the module and constant rather
  than letting the surface go unaccounted

#### Scenario: Inventory row drifts from the constant

- **WHEN** an inventory row's recorded version no longer equals the live
  constant's value
- **THEN** the agreement test fails naming both values

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

### Requirement: Single-sourced secret field vocabulary

Forge SHALL consume the contract registry's secret-field name list as the
authoritative vocabulary for field names in shared documents, SHALL union it
with its existing value-shape redaction rather than replacing it, and SHALL
drop no previously redacted term.

#### Scenario: Consumed field name is refused in a document

- **WHEN** a document presented for validation carries a field name containing a
  registered secret substring
- **THEN** validation fails naming the field path, and the value is never
  echoed back

#### Scenario: Existing redaction still applies

- **WHEN** captured runtime output carries a credential shape Forge already
  redacted before this capability existed
- **THEN** it is still replaced, and a key whose name matches only the consumed
  list is now replaced as well

#### Scenario: Authoritative list stated once

- **WHEN** a reader asks which vocabulary governs field names versus captured
  output
- **THEN** the module documentation identifies the contract registry for field
  names and Forge's redactor for captured runtime output

### Requirement: Forge-owned digest-pinned contract assets

Forge SHALL maintain under `contracts/` only the platform schema families it
actively consumes, plus the shared envelope, registry data, fixtures, and
vocabularies required to validate those families. The active families are
`platform.gate-result`, `platform.readiness`, `platform.release-evidence`,
`platform.capability`, `platform.audit-event`, and
`platform.public-portfolio-manifest`. Forge SHALL hash every retained contract
asset in `contracts/manifest.json` and verify those digests offline without a
sibling checkout or network. The manifest SHALL identify Forge ownership and
SHALL NOT claim that upstream parity was checked. An unlisted contract asset,
missing asset, malformed record, or digest mismatch SHALL fail closed.

#### Scenario: Maintained assets match the Forge manifest

- **WHEN** the offline contract check hashes every listed asset
- **THEN** each recorded SHA-256 matches and the report identifies Forge as
  owner

#### Scenario: Maintained schema changes without manifest update

- **WHEN** a retained schema or fixture differs from its recorded digest
- **THEN** the check fails naming the relative path and expected and actual
  digests

#### Scenario: Unlisted or missing contract asset

- **WHEN** an asset under the contract data roots is not listed, or a listed
  asset is missing or unreadable
- **THEN** the check fails naming the path and does not report success

#### Scenario: No sibling source is available

- **GIVEN** no `platform-contracts` sibling checkout and no
  `PLATFORM_CONTRACTS_DIR`
- **WHEN** the contract check and Forge contract commands run
- **THEN** they operate from Forge-maintained assets without attempting a
  network or sibling-path lookup

### Requirement: Contract projection stays read-only on every exposed surface

Forge SHALL keep `forge contract list`, `inspect`, `emit`, and `validate`
read-only with respect to project and journal state. The CLI and authenticated
assurance API SHALL expose the existing Forge contract inventory and platform
family inspection. The platform family catalog SHALL be additive; no MCP
mutation tool or unauthenticated assurance disclosure SHALL be introduced.

#### Scenario: Existing contract inventory remains read-only

- **WHEN** an operator lists or inspects contract records
- **THEN** the output is derived from Forge-owned records and no files, journal
  rows, or project state are changed

#### Scenario: Assurance catalog remains authenticated

- **WHEN** an unauthenticated caller requests platform family list or detail
- **THEN** the route returns the existing unauthorized response without
  revealing the catalog

### Requirement: Additive platform-family catalog compatibility boundary

Forge SHALL keep retained family wire shapes, discriminators, supported major
versions, existing CLI contract inventory rows, and existing `contracts` and
`manifest` JSON response fields compatible. The CLI and authenticated
assurance API SHALL expose a separate `platform_families` catalog containing
supported and explicitly unsupported families.

#### Scenario: Existing inventory consumers remain compatible

- **WHEN** an existing consumer reads `forge contract list --format json` or
  `GET /v1/admin/contracts`
- **THEN** existing `contracts` rows and `manifest` retain their prior shapes,
  and the additive `platform_families` field reports family support status

#### Scenario: Unsupported family is visible and cannot be used

- **WHEN** a maintainer lists contract families and selects an unsupported
  family
- **THEN** both CLI and authenticated assurance surfaces show its status and
  reason, and inspect, emit, and validate return a typed unsupported response

### Requirement: Declared contract version per supported family

Forge SHALL register, in one reviewed table, the wire version it emits for
every supported platform envelope family, SHALL build each envelope's
`contract` discriminator from that table rather than from a literal, and SHALL
derive `forge contract inspect`'s declared version from the same table. The
table SHALL cover exactly the five supported envelope families with no
duplicates; every value SHALL match the platform contract pattern and the
Forge-maintained registry SHALL still serve its major. The public portfolio
manifest family SHALL be listed as supported with its schema path and version,
and SHALL be validated against that schema by the publisher's normal test
suite.

#### Scenario: Emitted envelope declares the registered version

- **WHEN** an existing supported family is emitted
- **THEN** its envelope discriminator and inspect output use the same
  registered version

#### Scenario: Family support records and schema set agree

- **WHEN** the family catalog is loaded
- **THEN** each supported family has exactly one existing schema and required
  version record, and no unsupported family is resolvable as a schema

#### Scenario: Portfolio publisher validates its output

- **WHEN** the ordinary contract test suite publishes a portfolio manifest
- **THEN** the in-tree Draft 2020-12 schema accepts it and rejects the invalid
  canonical fixtures

#### Scenario: Unsupported major is refused

- **WHEN** a supported family's envelope uses a major absent from the
  Forge-maintained registry
- **THEN** validation refuses the family and version rather than accepting a
  fallback

### Requirement: Unsupported contract families are explicit

Forge SHALL maintain an explicit family support catalog. Each supported family
SHALL identify its schema path and, for envelope families, a registered wire
version. Each unsupported family SHALL identify a non-empty reason and SHALL
not resolve to a schema path. The catalog SHALL include
`platform.identity-subject`, `platform.permission-decision`,
`platform.tenant-context`, and `platform.job-outcome` as unsupported until a
Forge implementation change adds an actual consumer, validator, and/or
producer appropriate to that family.

#### Scenario: Unsupported families are listed with reasons

- **WHEN** the catalog is loaded and rendered in CLI or authenticated web
  output
- **THEN** all four named families appear with `unsupported` status and a
  non-empty actionable reason

#### Scenario: Unsupported family has no validator or emitter fallback

- **WHEN** inspect, validate, or emit receives an unsupported family
- **THEN** Forge returns a typed unsupported-family result before reading a
  target or emitting a document

#### Scenario: Unknown family remains distinct

- **WHEN** a family absent from both supported and unsupported catalog entries
  is requested
- **THEN** Forge returns the existing bounded unknown-family result

### Requirement: CI and release validation are offline

Forge CI and `scripts/release-check.sh` SHALL run the same manifest, family
catalog, schema, and fixture validation without checking out
`platform-contracts`. A missing validator dependency SHALL fail with an
actionable install command; it SHALL NOT be skipped or downgraded to a note.

#### Scenario: Required CI has no source checkout

- **WHEN** Forge CI runs from a clean checkout
- **THEN** schema, fixture, catalog, and manifest validation runs without
  fetching or requiring a sibling repository

#### Scenario: Release check is offline

- **WHEN** `scripts/release-check.sh` runs with no external contract directory
- **THEN** it executes the same offline contract validation and reports the
  concrete failing asset or fixture

#### Scenario: Required validator is unavailable

- **WHEN** the schema validator is not installed
- **THEN** the check fails and prints the pinned development dependency install
  command

### Requirement: Forge contracts are not declared as a sibling provider copy

Forge SHALL own its retained contract assets and vocabulary within the Forge
project declaration and contract manifest. Forge SHALL NOT declare
`contracts/manifest.json` or its retained schemas as a consumed copy provided
by the independent `platform-contracts` project.

#### Scenario: Project declaration describes Forge ownership

- **WHEN** the Forge project interface declaration is inspected
- **THEN** its retained contract assets are accounted for by Forge's own
  manifest and no required `platform-contract-copy` provider interface exists
