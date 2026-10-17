## ADDED Requirements

### Requirement: Generated projects receive profile-compatible Forge Gate entry points

For each current Forge project profile, Forge SHALL select the verified
profile-compatible standard pack by default when no explicit pack is
specified. The service profiles SHALL receive the current `baseline-service`
pack, including its shared language verifier. The client profiles SHALL
receive the current Gate-only `baseline-client` pack until a native language
verifier has been verified for those profiles. Every pack SHALL include a
receipt for its owned assets. Explicit supported `--standard-pack` selections
SHALL remain honored, and unsupported future profiles SHALL not receive an
unverified pack implicitly.

#### Scenario: New service project gets CI and pre-commit templates

- **WHEN** a new project is generated for `aspnet-web`, `nextjs-web`,
  `python-service`, or `rust-web` without an explicit standard-pack selector
- **THEN** Forge renders the current supported `baseline-service` pack,
  including its standard verifier, active GitHub Actions workflow,
  pre-commit configuration, and receipt

#### Scenario: New client project gets Gate-only templates

- **WHEN** a new project is generated for `react-web` or `flutter-app` without
  an explicit standard-pack selector
- **THEN** Forge renders the current supported `baseline-client` pack,
  including its Gate verification entry point, active GitHub Actions workflow,
  pre-commit configuration, and receipt

#### Scenario: Explicit older compatible pack remains pinned

- **WHEN** a compatible project is generated with an explicit supported pack
  version
- **THEN** Forge renders that exact version and does not silently substitute a
  newer pack

#### Scenario: Future unsupported profile does not receive a guessed pack

- **WHEN** a project profile is outside all verified pack compatibility sets
- **THEN** Forge does not render a pack implicitly

### Requirement: CI and pre-commit run the same Forge Gate

Each current supported pack SHALL provide an active GitHub Actions workflow at
`.github/workflows/forge-gate.yml` and a pre-commit hook that both run the
Forge Gate repository-wide. The service CI SHALL also run its language-aware
standard verifier; the client CI SHALL run its Gate verification entry point.
CI SHALL install a Gate-capable Forge revision immutably and use read-only
repository permissions. Neither entry point SHALL swallow a failing status or
suppress project Gate extensions.

#### Scenario: Service CI runs verifier and Gate

- **WHEN** a generated or upgraded service project workflow runs
- **THEN** it executes `.standard/scripts/verify.sh` and `forge gate .`, and a
  failure from either step fails the workflow

#### Scenario: Client CI runs the Gate entry point

- **WHEN** a generated or upgraded client project workflow runs
- **THEN** it executes `.standard/scripts/verify.sh` whose command is
  `forge gate .`, and a Gate failure fails the workflow

#### Scenario: Pre-commit runs the same repository-wide Gate

- **WHEN** a developer runs the configured pre-commit hook
- **THEN** it runs `forge gate .` without narrowing checks to the hook's
  changed-file arguments

#### Scenario: Gate extension failure remains blocking

- **WHEN** a project's registered Gate extension fails under either entry
  point
- **THEN** the CI job or pre-commit hook returns failure while preserving the
  Forge mandatory baseline

#### Scenario: Required Forge installation is unavailable

- **WHEN** CI cannot install the pinned Forge revision or the local hook cannot
  resolve Forge on `PATH`
- **THEN** the corresponding entry point fails and reports the missing
  prerequisite rather than reporting success

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
