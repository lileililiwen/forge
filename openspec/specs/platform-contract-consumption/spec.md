# platform-contract-consumption Specification

## Purpose
TBD - created by archiving change platform-contract-consumption. Update Purpose after archive.
## Requirements
### Requirement: Digest-pinned vendored contract set

Forge SHALL vendor the published contract schemas, the contract registry and the
secret-field name list under `contracts/` behind a `contracts/manifest.json`
that records the source revision and the sha256 of every vendored file, and
SHALL verify those digests offline without reaching a sibling checkout or the
network.

#### Scenario: Vendored bytes match the manifest

- **WHEN** the digest test hashes every file listed in `contracts/manifest.json`
- **THEN** each recorded sha256 matches and the recorded source revision is
  reported

#### Scenario: Vendored file was edited locally

- **WHEN** a vendored schema's bytes differ from its recorded digest
- **THEN** the check fails naming the file, the expected digest and the actual
  digest, and no contract surface silently accepts the edited schema

#### Scenario: Contract set absent

- **WHEN** `contracts/manifest.json` is missing or unreadable and an operator
  requests a contract surface
- **THEN** the command refuses with a typed contract-invalid error naming the
  exact path, and no embedded or cached copy of the schemas is used instead

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

### Requirement: Read-only contract projection surface

Forge SHALL provide `forge contract list`, `forge contract inspect <family>`,
`forge contract emit <family> [TARGET]` and `forge contract validate <file|->`
in human and JSON form, and `emit` SHALL derive its envelope from an existing
Core record without writing files, journaling operations or mutating state.

#### Scenario: Emit gate result from persisted evidence

- **WHEN** a project has fresh revision-bound gate evidence and
  `forge contract emit gate-result <target>` runs
- **THEN** one `platform.gate-result/0.1.0` envelope prints, validates against
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

### Requirement: Refusing status mapping

Forge SHALL map its own status vocabularies to contract status values only
through a total mapping that returns an explicit refusal for any value with no
contract equivalent, and SHALL NOT default an unmapped, unverified or unknown
value into a passing or healthy contract status.

#### Scenario: Mapped verdicts project

- **WHEN** a gate aggregate of passed, blocked or failed is projected
- **THEN** the envelope carries respectively `passed`, `failed` or `errored` and
  the Forge record itself is unchanged

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
- **THEN** the projection reports the non-passing interpretation, mirroring the
  gate plane's existing downgrade rule

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

### Requirement: Additive-only compatibility boundary

Forge SHALL keep every existing document, discriminator, exit code, journal row
and generated tree byte-identical when this capability is present, and SHALL
limit contract surfaces to the CLI, exposing no MCP tool, API route or portal
section.

#### Scenario: Existing surfaces unchanged

- **WHEN** gate status JSON, the checker document, the fleet listing and the
  provider matrix are captured before and after this capability is added
- **THEN** the captured bytes are identical

#### Scenario: Dependency graph of the shipped binary

- **WHEN** the workspace dependency manifest is inspected
- **THEN** no schema-validation dependency appears in the runtime dependency
  set, only in the development set

#### Scenario: Transports advertise nothing new

- **WHEN** the mature MCP tool registry, the API route table and the portal
  section set are listed
- **THEN** no contract surface appears in any of them

