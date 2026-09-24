# deterministic-project-generation Specification

## ADDED Requirements

### Requirement: Workspace-compatible project metadata

Generation SHALL write a sibling-schema-compatible `.project.json` carrying
the project id, a governance profile from an explicit per-profile mapping,
the profile's native verification command, `evidence_status: planned`, and
non-deployable defaults, and SHALL omit the file — never guess — when the
profile has no mapping or the operator opts out.

#### Scenario: Mapped profile generated

- **WHEN** `forge new` creates a project from a profile with a governance
  mapping
- **THEN** the output tree contains `.project.json` whose verification
  command equals the profile's native test command and whose
  evidence_status is planned

#### Scenario: Unmapped profile

- **WHEN** the chosen profile declares no governance mapping
- **THEN** no metadata file is written, a note names the missing mapping,
  and every other generation output is unchanged

#### Scenario: Opt-out parity

- **WHEN** generation runs with `--no-workspace-metadata`
- **THEN** the output is byte-identical to the pre-change release

### Requirement: Metadata ownership and honesty

The generated metadata file SHALL be covered by ownership receipts so user
edits survive upgrades as conflicts-for-review rather than silent
overwrites, SHALL never record evidence it did not observe, and import
SHALL only observe an existing file without writing or rewriting it.

#### Scenario: Edited metadata at upgrade

- **WHEN** a project's `.project.json` was user-modified and an upgrade
  would change it
- **THEN** the upgrade refuses with the ownership-conflict code and leaves
  the edited file untouched

#### Scenario: Import preserves foreign metadata

- **WHEN** an imported project already carries `.project.json`
- **THEN** its bytes are unchanged and its presence is recorded as
  informational evidence only
