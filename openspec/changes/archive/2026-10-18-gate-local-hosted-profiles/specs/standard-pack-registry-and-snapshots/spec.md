# standard-pack-registry-and-snapshots Specification

## MODIFIED Requirements

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
