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

### Requirement: Generated projects receive profile-compatible Forge Gate entry points

For each current Forge project profile, Forge SHALL select the verified profile-compatible standard pack by default when no explicit pack is specified. The service profiles SHALL receive the current `baseline-service` pack, including its shared language verifier. The client profiles SHALL receive the current Gate-only `baseline-client` pack until a native language verifier has been verified for those profiles. Every pack SHALL include a receipt for its owned assets. Explicit supported `--standard-pack` selections SHALL remain honored, and unsupported future profiles SHALL not receive an unverified pack implicitly.

#### Scenario: New service project gets CI and pre-commit templates

- **WHEN** a new project is generated for `aspnet-web`, `nextjs-web`, `python-service`, or `rust-web` without an explicit standard-pack selector
- **THEN** Forge renders the current supported `baseline-service` pack, including its standard verifier, active GitHub Actions workflow, pre-commit configuration, and receipt

#### Scenario: New client project gets Gate-only templates

- **WHEN** a new project is generated for `react-web` or `flutter-app` without an explicit standard-pack selector
- **THEN** Forge renders the current supported `baseline-client` pack, including its Gate verification entry point, active GitHub Actions workflow, pre-commit configuration, and receipt

#### Scenario: Explicit older compatible pack remains pinned

- **WHEN** a compatible project is generated with an explicit supported pack version
- **THEN** Forge renders that exact version and does not silently substitute a newer pack

#### Scenario: Future unsupported profile does not receive a guessed pack

- **WHEN** a project profile is outside all verified pack compatibility sets
- **THEN** Forge does not render a pack implicitly

### Requirement: CI and pre-commit run the same Forge Gate

Each current supported standard pack SHALL provide an active GitHub Actions workflow at `.github/workflows/forge-gate.yml` and a pre-commit hook that both run the Forge Gate repository-wide. The current default service pack `baseline-service@1.6.0` and client pack `baseline-client@1.3.0` SHALL install Forge `0.1.1` from its versioned GitHub archive and SHALL verify the SHA-256 sidecar before extraction. The service CI SHALL also run its language-aware standard verifier; the client CI SHALL run its Gate verification entry point. Consumer CI SHALL use read-only repository permissions, SHALL not compile Forge from a source checkout, and SHALL not resolve an unpinned latest release. Pre-commit SHALL invoke `forge gate . --profile fast`; CI SHALL invoke `forge gate . --profile full`. Both profiles SHALL retain the mandatory Forge baseline, while only full CI runs project extensions. Neither entry point SHALL swallow a failing status. Older supported pack versions remain immutable and continue to render their recorded bytes.

#### Scenario: Service CI runs verifier and Gate

- **WHEN** a generated or upgraded service project uses the current default pack and its pinned release is available
- **THEN** it downloads the exact release assets, verifies the digest, installs Forge, executes `.standard/scripts/verify.sh` and `forge gate . --profile full`, and a failure from any step fails the workflow

#### Scenario: Client CI runs the Gate entry point

- **WHEN** a generated or upgraded client project uses the current default pack
- **THEN** it downloads and verifies the exact release assets, installs Forge, executes `.standard/scripts/verify.sh` with the full profile, and a Gate failure fails the workflow

#### Scenario: Pre-commit runs the same repository-wide Gate

- **WHEN** a developer runs the configured pre-commit hook
- **THEN** it runs `forge gate . --profile fast` repository-wide, preserves the mandatory baseline, and does not report omitted project extensions as passing

#### Scenario: Gate extension failure remains blocking

- **WHEN** a project's registered Gate extension fails in hosted CI
- **THEN** the CI job returns failure while preserving the Forge mandatory baseline

#### Scenario: Release asset cannot be downloaded or verified

- **WHEN** either release asset is absent, malformed, names a different archive or fails checksum verification
- **THEN** CI fails before extraction and does not invoke Forge Gate

#### Scenario: Required Forge installation is unavailable

- **WHEN** CI cannot download or install its pinned Forge release, or the local hook cannot resolve Forge on `PATH`
- **THEN** the corresponding entry point fails and reports the missing prerequisite instead of reporting success

#### Scenario: Older supported pack remains byte-stable

- **WHEN** Forge renders an explicit older supported pack version
- **THEN** its original workflow, installer behavior and asset receipt bytes remain unchanged

#### Scenario: Local hook cannot resolve Forge

- **WHEN** the local hook cannot resolve Forge on `PATH`
- **THEN** the hook fails and reports the missing prerequisite rather than reporting success

### Requirement: Legacy projects adopt current templates conflict-safely

An existing compatible project SHALL be able to adopt its profile-compatible
CI and pre-commit assets through the existing explicit standard-pack
installation or upgrade. Forge SHALL preserve modified owned files and
unrelated project files, and must not silently overwrite a pre-existing CI or
pre-commit configuration.

#### Scenario: Legacy project installs templates

- **WHEN** an operator installs the current supported pack into a compatible
  project with no conflicting owned paths
- **THEN** Forge writes the owned templates and receipt and the project can run
  both Gate entry points

#### Scenario: Existing workflow or hook conflicts

- **WHEN** a target CI or pre-commit path already exists with bytes different
  from the rendered pack
- **THEN** Forge names the conflicting path and writes no project files
