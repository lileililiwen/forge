# deterministic-project-generation Specification

## Purpose
Deterministic project creation through forge new with explicit or interactive profile selection, rendering pinned native-buildable templates that produce equivalent ordinary source from identical inputs and keep working without Forge.
## Requirements
### Requirement: Equivalent deterministic creation modes

Forge SHALL implement forge new with explicit profile selection and interactive input, producing equivalent ordinary source from identical inputs and pinned assets.

#### Scenario: Equivalent deterministic creation modes success

- **WHEN** the same creation request is supplied by flags and interactive answers
- **THEN** both outputs have equivalent file contents except documented identity fields

#### Scenario: Equivalent deterministic creation modes failure

- **WHEN** creation targets a nonempty directory or a template escapes its destination
- **THEN** creation fails before overwriting existing files

#### Scenario: Equivalent deterministic creation modes boundary

- **WHEN** creation is cancelled during interactive input
- **THEN** no project or registry entry is left behind

### Requirement: Portable generated projects

Forge SHALL generate the five MVP profile projects with manifests and documented native build/test commands, without a Forge runtime dependency.

#### Scenario: Portable generated projects success

- **WHEN** a generated fixture is built using its stack toolchain with Forge unavailable
- **THEN** the native build and applicable tests succeed

#### Scenario: Portable generated projects failure

- **WHEN** generation or validation fails before promotion
- **THEN** the destination is not registered as a completed project and staged output is recoverable or cleaned

#### Scenario: Portable generated projects boundary

- **WHEN** a host lacks a target stack toolchain
- **THEN** Forge reports that native validation is unverified and does not claim a successful build

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

