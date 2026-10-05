# portfolio-share Specification

## ADDED Requirements

### Requirement: The public manifest is emitted in the contracted shape

The system SHALL emit the public manifest document in the shape defined by the
pinned `platform.public-portfolio-manifest` contract, and SHALL refuse to
invent, rename or renumber a contracted field. Specifically: `schema_family`
SHALL be the string `platform.public-portfolio-manifest`; `schema_version`
SHALL be a string matching `^[0-9]+\.[0-9]+\.[0-9]+$` carrying the full
contract version Forge produces; and `manifest_revision` SHALL be a string
between 1 and 64 characters matching `^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$`,
derived deterministically from the internal revision.

The internal revision SHALL remain an integer in the registry column, the
approval, the audit trail, the publication report and the adapter envelope. The
wire encoding SHALL be a pure, injective function of that integer, so that the
encoding is stable across runs and platforms, no two revisions collide, and an
unchanged catalog keeps hashing identically.

#### Scenario: A published document satisfies the contract

- **GIVEN** an approved catalog of share records
- **WHEN** the manifest document is published
- **THEN** the serialized document validates against
  `public-portfolio-manifest.schema.json`
- **AND** `schema_family` is `platform.public-portfolio-manifest`
- **AND** `schema_version` is the string `1.0.0`
- **AND** `manifest_revision` is the string `rev_<revision>` for that catalog's
  revision

#### Scenario: The revision encoding matches the contract's pattern

- **WHEN** the revision `0` is encoded
- **THEN** the result is `rev_0`, which matches
  `^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$` and is 1 to 64 characters long
- **AND** encoding the largest representable `u32` revision still produces a
  value inside those bounds

#### Scenario: Re-exporting an unchanged catalog is byte-identical

- **GIVEN** a catalog that has not changed
- **WHEN** the manifest is built and hashed twice
- **THEN** both `canonical_json()` and `manifest_sha256()` are identical
- **AND** neither the emission time nor the hash itself takes part in the hashed
  body

#### Scenario: Persistence and the audit trail keep the integer

- **GIVEN** a publication attempt for revision `4`
- **WHEN** the publication report, the adapter envelope and the audit trail are
  inspected
- **THEN** each records the integer `4` and none of them records the wire
  encoding
- **AND** the registry column `manifest_revision` remains `INTEGER`

#### Scenario: An approval made before the encoding change

- **GIVEN** an approval whose `manifest_sha256` was computed from the previous
  wire shape
- **WHEN** publication is attempted
- **THEN** publication is refused naming both hashes, and the operator's remedy
  is to preview and approve the revision again
- **AND** no stored approval or audit entry is rewritten to match
