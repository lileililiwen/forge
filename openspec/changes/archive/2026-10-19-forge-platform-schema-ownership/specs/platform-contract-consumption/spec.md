# platform-contract-consumption Specification

## REMOVED Requirements

### Requirement: Digest-pinned vendored contract set

**Reason:** Forge now maintains the schemas it actively consumes under its own
review and compatibility policy. Its content manifest identifies Forge as the
owner and no longer claims source revision parity.

### Requirement: Read-only contract projection surface

**Reason:** The old CLI-only boundary is obsolete. Contract inventory and
platform-family inspection already have authenticated assurance API and web
surfaces, which this change extends additively.

### Requirement: Additive-only compatibility boundary

**Reason:** This requirement described the original CLI-only contract feature.
It is replaced with compatibility guarantees for the existing CLI/API fields
and the additive supported/unsupported platform-family catalog.

### Requirement: Declared contract version per family

**Reason:** Forge now has five supported envelope families and one supported
standalone portfolio manifest. The revised requirement distinguishes their
versioning and schema behavior.

### Requirement: The parity gate compares the vendored families against the contract source

**Reason:** Forge no longer claims whole-catalog parity with the independent
Platform Contracts project.

### Requirement: A parity check that compared nothing is not a pass

**Reason:** The source-to-mirror parity check is removed. Offline manifest and
schema validation provide Forge's integrity oracle.

### Requirement: An unresolvable contract source is a hard failure

**Reason:** Forge contract operations no longer resolve an external source.

### Requirement: No input may weaken the comparison

**Reason:** No source comparison or `PLATFORM_CONTRACTS_DIR` comparison mode
remains in Forge.

### Requirement: The parity gate blocks release and CI

**Reason:** CI and release checks invoke the offline Forge-owned contract
validator rather than a sibling-source parity gate.

## ADDED Requirements

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
