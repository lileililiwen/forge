# profile-and-release-readiness Specification

## Purpose
Reproducible release-readiness evidence: native build/test matrix rows per supported profile with missing toolchains classified unverified (never passing), artifact checksums with version smoke, and a gate that passes only when every selected row passes. Every native command a row runs goes through the shared native command runner, under its wall-clock bound, so a row never hangs on an unresponsive command.
## Requirements
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

### Requirement: Reproducible release gate and artifact smoke evidence

Forge SHALL provide CI/local-equivalent checks for source quality, strict
OpenSpec validation and the native profile matrix, plus a verifiable native
Forge artifact with version and checksum evidence.

#### Scenario: Gate passes

- **WHEN** all required checks and supported profile rows pass
- **THEN** the artifact is eligible for the existing gated release workflow

#### Scenario: Gate blocks

- **WHEN** a required check fails or a required profile row is unverified
- **THEN** release readiness is blocked with the exact failed or unavailable check

#### Scenario: Artifact remains ordinary

- **WHEN** a generated project is run after Forge is absent from PATH
- **THEN** its native build/test behavior remains unchanged

### Requirement: Declared commands match the generated tree

A supported profile's advertised build and test commands SHALL be commands the
tree that profile generates can execute, and no scaffold verification command
SHALL depend on a platform host, a browser environment or a package install that
the template does not itself provide.

#### Scenario: Scaffold verifies with Forge absent

- **WHEN** a generated project's declared build and test commands run in a
  disposable fixture with Forge absent from `PATH`
- **THEN** both exit zero and the readiness row records the toolchain version,
  source digest and timestamp for that run

#### Scenario: Template declares a browser-free Node test

- **WHEN** a generated JavaScript profile's test command runs under the bare
  platform test runner
- **THEN** it passes while still asserting the template's real contract, and a
  broken template still fails the row

#### Scenario: Template declares no platform host

- **WHEN** a client profile ships source without a platform host project
- **THEN** its scaffold verification command performs work the shipped tree
  supports, and the platform bundle command remains an operator-level release
  command rather than a claimed scaffold guarantee

#### Scenario: Runner prerequisite named

- **WHEN** a declared command depends on a module or tool the runner may not
  carry
- **THEN** the prerequisite is recorded in the profile's qualification note, and
  a host without it reports `unverified` naming the tool instead of a code failure

### Requirement: Command changes keep generated metadata and ownership truthful

Changing a profile's declared commands SHALL change the generated project
declaration, its ownership receipt and its upgrade behaviour consistently, and
SHALL never rewrite a declaration a user edited.

#### Scenario: Unedited stale declaration refreshes

- **WHEN** an upgrade runs against a project whose declaration predates the
  command change and whose files are unedited
- **THEN** the declaration and receipt refresh together and both files are named
  in the change report

#### Scenario: Edited declaration is preserved

- **WHEN** the same upgrade meets a user-edited declaration
- **THEN** it refuses with the ownership conflict and leaves the declaration and
  manifest byte-preserved

#### Scenario: Foreign declaration stays foreign

- **WHEN** a declaration exists without a Forge receipt
- **THEN** inspect, adopt and upgrade leave it untouched and invent no receipt

#### Scenario: Emitted command matches the sibling's expectations

- **WHEN** a freshly generated project is checked by Workspace Governance's
  declaration audit
- **THEN** it reports no verification-command finding because the emitted command
  is the descriptor's real test command

### Requirement: Full-matrix gate claim is evidence-gated

The readiness gate SHALL report a full-matrix pass only when every supported
profile row passes on the runner executing it, SHALL keep rows the runner cannot
qualify as `unverified`, and SHALL never convert a failing or unverified row into
a pass or a skip.

#### Scenario: Every row passes

- **WHEN** the matrix runs on a runner that supplies every supported profile's
  prerequisites
- **THEN** the unqualified gate exits zero and names each row's evidence

#### Scenario: Runner lacks one toolchain

- **WHEN** one supported profile's toolchain is absent
- **THEN** that row is `unverified`, the full-matrix gate does not report a pass,
  and the qualified subset verdict remains available with the excluded row
  visible

#### Scenario: Subset qualification is honest

- **WHEN** CI qualifies a subset of profiles
- **THEN** the CI job names the versions it installs and the evidence records the
  same versions rather than the ones a different host happened to qualify

