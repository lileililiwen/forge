# standard-pack-registry-and-snapshots Specification

## Purpose
TBD - created by archiving change standard-pack-registry-and-snapshots. Update Purpose after archive.
## Requirements
### Requirement: Versioned standard packs

Forge SHALL expose versioned standard-pack descriptors with profile
compatibility, support state, asset digest, and deterministic render inputs.

#### Scenario: Supported pack listed

- **WHEN** an operator runs `forge standard list`
- **THEN** supported packs include identity, version, compatible profiles, and
  evidence state

#### Scenario: Unsupported pack refused

- **WHEN** an operator selects an unknown or unsupported pack
- **THEN** Forge returns a typed refusal and changes no project files

### Requirement: Standalone snapshot generation

Forge SHALL materialize a repository-local standard snapshot, verification
entry point, CI/quality configuration, Compose selection, and ownership receipt
without requiring Forge or a sibling checkout for later project operation.

#### Scenario: Snapshot generated

- **WHEN** a supported profile and pack are selected during project generation
- **THEN** the expected local files and digest receipt are written atomically

#### Scenario: External pack unavailable

- **WHEN** an optional external template source is unavailable
- **THEN** Forge uses the declared local fallback or returns typed unavailable;
  it never silently fetches or invents files

### Requirement: Explicit conflict-safe upgrades

Forge SHALL provide standard diff and explicit upgrade operations that preserve
unrelated files and refuse modified owned files unless a reviewable conflict
resolution is supplied.

#### Scenario: Unmodified snapshot upgrade

- **WHEN** the receipt matches the existing owned files and a compatible newer
  pack is selected with confirmation
- **THEN** Forge updates only owned snapshot files and refreshes the receipt

#### Scenario: Modified owned file

- **WHEN** an owned generated file differs from its receipt
- **THEN** Forge reports a conflict and writes no replacement
