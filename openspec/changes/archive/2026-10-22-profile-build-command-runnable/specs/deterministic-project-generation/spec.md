## MODIFIED Requirements

### Requirement: Portable generated projects

Forge SHALL generate the five MVP profile projects with manifests and documented native build/test commands, without a Forge runtime dependency. Each documented native build command SHALL run with the stack toolchain the profile declares.

#### Scenario: Portable generated projects success

- **WHEN** a generated fixture is built using its stack toolchain with Forge unavailable
- **THEN** the native build and applicable tests succeed

#### Scenario: Portable generated projects failure

- **WHEN** generation or validation fails before promotion
- **THEN** the destination is not registered as a completed project and staged output is recoverable or cleaned

#### Scenario: Portable generated projects boundary

- **WHEN** a host lacks a target stack toolchain
- **THEN** Forge reports that native validation is unverified and does not claim a successful build

#### Scenario: The documented build command runs with the stack toolchain

- **WHEN** a generated fixture is built with its stack toolchain alone
- **THEN** the profile's declared build command succeeds without a
  network install and without a package outside that toolchain
