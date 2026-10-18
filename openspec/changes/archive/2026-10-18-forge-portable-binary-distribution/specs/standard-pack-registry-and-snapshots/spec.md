## MODIFIED Requirements

### Requirement: CI and pre-commit run the same Forge Gate

Each supported standard pack SHALL provide an active GitHub Actions workflow
at `.github/workflows/forge-gate.yml` and a pre-commit hook that both run the
Forge Gate repository-wide. The current default service pack
`baseline-service@1.4.0` and client pack `baseline-client@1.1.0` SHALL install
the exact pinned Forge release from its versioned GitHub archive and SHALL
verify the SHA-256 sidecar before extraction. The service CI SHALL also run its
language-aware standard verifier; the client CI SHALL run its Gate verification
entry point. Consumer CI SHALL use read-only repository permissions, SHALL
not compile Forge from a source checkout, and SHALL not resolve an unpinned
latest release. Neither entry point SHALL swallow a failing status or suppress
project Gate extensions. Older supported pack versions remain immutable and
continue to render their recorded bytes.

#### Scenario: Service CI runs verifier and Gate

- **WHEN** a generated or upgraded service project uses the current default
  pack and its pinned release is available
- **THEN** it downloads the exact release assets, verifies the digest, installs
  Forge, executes `.standard/scripts/verify.sh` and `forge gate .`, and a
  failure from any step fails the workflow

#### Scenario: Client CI runs the Gate entry point

- **WHEN** a generated or upgraded client project uses the current default
  pack and its pinned release is available
- **THEN** it downloads and verifies the exact release assets, installs Forge,
  executes `.standard/scripts/verify.sh` whose command is `forge gate .`, and
  a Gate failure fails the workflow

#### Scenario: Pre-commit runs the same repository-wide Gate

- **WHEN** a developer runs the configured pre-commit hook
- **THEN** it runs `forge gate .` without narrowing checks to the hook's
  changed-file arguments

#### Scenario: Gate extension failure remains blocking

- **WHEN** a project's registered Gate extension fails under either entry
  point
- **THEN** the CI job or pre-commit hook returns failure while preserving the
  Forge mandatory baseline

#### Scenario: Release asset cannot be downloaded or verified

- **WHEN** either release asset is absent, malformed, names a different
  archive or fails checksum verification
- **THEN** CI fails before extraction and does not invoke Forge Gate

#### Scenario: Required Forge installation is unavailable

- **WHEN** CI cannot download or install its pinned Forge release, or the local
  hook cannot resolve Forge on `PATH`
- **THEN** the corresponding entry point fails and reports the missing
  prerequisite instead of reporting success

#### Scenario: Older supported pack remains byte-stable

- **WHEN** Forge renders an explicit older supported pack version
- **THEN** its original workflow, installer behavior and asset receipt bytes
  remain unchanged

#### Scenario: Local hook cannot resolve Forge

- **WHEN** the local hook cannot resolve Forge on `PATH`
- **THEN** the hook fails and reports the missing prerequisite rather than
  reporting success
