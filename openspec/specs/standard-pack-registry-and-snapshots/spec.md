# standard-pack-registry-and-snapshots Specification

## Purpose
Versioned standard packs plus standalone snapshot generation and explicit conflict-safe upgrades.
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

### Requirement: Shared verification scripts travel as versioned pack content

The baseline standard pack SHALL carry the workspace's shared verification
scripts as versioned assets with recorded digests: the language-detecting CI
rehearsal script as the snapshot verification entry point, and the OpenSpec
change-name checker plus the concern-check shims at the repository-relative
paths the workspace Gate generator binds. The content SHALL be authored by the
workspace governance owner and pinned into this repository as bundled bytes; it
SHALL never be fetched at run time, so a snapshot renders identically with no
sibling checkout present. The verification entry point SHALL be the real
rehearsal script and SHALL NOT be a stub. Every asset SHALL be committed as a
reviewable file rather than embedded in a source-code string literal.

#### Scenario: Pack descriptor lists the shared assets

- **WHEN** an operator runs `forge standard inspect baseline-service@1.2.0`
- **THEN** the descriptor lists the snapshot verification script, the
  change-name checker and the concern shims among its assets, and its asset
  digest covers their pinned bytes

#### Scenario: Rendered asset is the pinned canonical file

- **WHEN** the pack is rendered for a project
- **THEN** the snapshot verification script is byte-equal to the pinned
  canonical rehearsal script, and the change-name checker is byte-equal to the
  canonical checker, so a project that installs the pack runs the same logic the
  hand-copied files ran

#### Scenario: A stub is replaced visibly

- **WHEN** a project pinned to the previous pack version diffs against the
  version carrying the shared assets
- **THEN** the verification script is reported as `updated`, the newly owned
  scripts as `added`, and every other owned file as `unchanged`

#### Scenario: New generation may select the pack

- **WHEN** an operator runs `forge new --standard-pack baseline-service@1.2.0`
  for a compatible profile
- **THEN** the generated project carries the shared assets and a receipt
  recording each one's digest

### Requirement: A project without a snapshot can install one

Forge SHALL provide an explicit installation operation that materializes a
standard snapshot into a project that has none, so an existing repository can
adopt pack-owned shared files instead of copying them. Installation SHALL
require explicit confirmation, SHALL refuse when the project already has a
snapshot receipt, and SHALL refuse when any pack-owned path already exists with
different bytes — naming every conflicting path and writing nothing. A
pack-owned path whose bytes already match SHALL be adopted without being
rewritten. Installation SHALL write only the rendered owned files and the
receipt, SHALL never modify an unrelated file, and SHALL offer no force mode:
replacing a repository's own file is a separate reviewed edit. Project identity
and profile SHALL be taken from the project's Forge manifest when present, and
otherwise from explicit arguments, and SHALL be recorded in the receipt.

#### Scenario: Installation requires confirmation

- **WHEN** an operator runs the installation without the confirmation flag
- **THEN** Forge refuses naming the flag, and no file in the project changes

#### Scenario: An existing snapshot is not overwritten

- **WHEN** the project already has a snapshot receipt
- **THEN** Forge refuses and directs the operator to the diff and upgrade
  operations; no file changes

#### Scenario: A divergent owned file blocks installation

- **WHEN** a pack-owned path already exists with different bytes
- **THEN** Forge refuses, names that path (and every other conflicting path), and
  writes nothing — the repository's own file is preserved

#### Scenario: An identical file is adopted untouched

- **WHEN** a pack-owned path already exists with bytes identical to the rendered
  content
- **THEN** Forge reports it as adopted and leaves its bytes unchanged

#### Scenario: Successful installation is verifiable

- **WHEN** an operator installs into a project with no conflicts and then runs
  `forge standard check`
- **THEN** the snapshot state is `rendered` with every owned file `present`, and
  a diff against the installed pack version reports no changes

#### Scenario: Only owned paths are written

- **WHEN** a project is installed into
- **THEN** the receipt and the rendered owned files are the only paths written;
  every other file in the project is byte-identical to before
