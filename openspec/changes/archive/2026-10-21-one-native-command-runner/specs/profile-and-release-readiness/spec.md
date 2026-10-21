## MODIFIED Requirements

### Requirement: Supported profile native evidence

Forge SHALL maintain a reproducible native evidence row for every supported
profile, including the five v0.1 profiles and `react-web`. Every native
command a row runs SHALL run through the shared native command runner, under
its wall-clock bound, so a row never hangs on an unresponsive command.

#### Scenario: Native evidence passes

- **WHEN** a generated fixture runs its declared native build and test commands without Forge
- **THEN** the row records toolchain versions, commands, source identity and a passed result

#### Scenario: Native command fails

- **WHEN** generation succeeds but a declared native build or test command fails
- **THEN** the profile is not reported as verified and the failure identifies the profile and command

#### Scenario: Toolchain is unavailable

- **WHEN** the host or CI runner lacks a required toolchain
- **THEN** the row is unverified with the missing prerequisite and cannot satisfy a release gate

#### Scenario: A native command overruns the bound

- **WHEN** a declared native build or test command is still running when the
  shared runner's bound elapses
- **THEN** the command's process group is killed, the row is not reported as
  verified, and the reason names the timeout
