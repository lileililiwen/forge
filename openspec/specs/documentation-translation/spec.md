# documentation-translation Specification

## Purpose
TBD - created by archiving change documentation-translation. Update Purpose after archive.
## Requirements
### Requirement: Canonical and derivative documentation

Forge SHALL implement docs translate with explicit locale enablement, derivative paths, source hashes and stale status on source changes.

#### Scenario: Canonical and derivative documentation success

- **WHEN** translation of an enabled locale completes against an unchanged source
- **THEN** the derivative records its source hash and review status

#### Scenario: Canonical and derivative documentation failure

- **WHEN** a derivative path resolves to the source file or outside the permitted project
- **THEN** translation fails before writing

#### Scenario: Canonical and derivative documentation boundary

- **WHEN** a locale is disabled
- **THEN** automatic workflows do not invoke its provider or create output

### Requirement: Incremental and reliable translation

Forge SHALL translate changed segments, preserve unchanged segments and technical literals, and leave existing outputs intact on provider failure.

#### Scenario: Incremental and reliable translation success

- **WHEN** only one source paragraph changes
- **THEN** the translation updates that segment and retains unchanged technical blocks

#### Scenario: Incremental and reliable translation failure

- **WHEN** the provider fails or the source changes during generation
- **THEN** the prior derivative remains intact and the attempt is failed or stale

#### Scenario: Incremental and reliable translation boundary

- **WHEN** the source hash is unchanged
- **THEN** translation reports current without repeating a full provider request

