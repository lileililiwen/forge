# target-addressed-build Specification

## Purpose
The spine of Forge has no build step. `forge test` runs the profile's test
command and the Gate has a `build` check, but nothing in Forge builds a project
for a target: the profile descriptor already carries a `build_command`, and
Forge Core has no operating-system, architecture or runtime-identifier
vocabulary at all. A project that must ship a binary for another platform has
no Forge operation for it, and the target the owner builds for is never
recorded. This capability gives the spine its build step and lets that step
address a target: the operating system and architecture, or the runtime
identifier, are resolved into the form the project's own toolchain reads, and
the resolved target is reported with the outcome.

## ADDED Requirements

### Requirement: A project has a build step

Forge SHALL build a project with the build command its profile declares, and
SHALL report the outcome including the command that ran, the status, the exit
code and the captured evidence.

#### Scenario: A build succeeds

- **WHEN** an owner builds a project whose profile declares a build command
- **THEN** the build runs that command in the project directory, and the
  outcome reports the command, the status `passed` and the exit code

#### Scenario: A failed build fails

- **WHEN** the build command exits with a non-zero status
- **THEN** the build is reported as a failure with the captured evidence, and
  no build is reported

#### Scenario: A profile without a build command

- **WHEN** an owner builds a project whose profile declares no build command
- **THEN** the build is refused and the refusal names the profile

### Requirement: A build may address a target

Forge SHALL accept an operating system with an architecture, or a runtime
identifier, and SHALL bind the build to that target through the mechanism the
project's toolchain reads. Forge SHALL refuse a target it cannot bind rather
than build for the host and report it as the requested target.

#### Scenario: A target build

- **WHEN** an owner builds with an operating system and an architecture for a
  project whose toolchain addresses a target
- **THEN** the build runs with that target bound, and the outcome names the
  resolved target

#### Scenario: A build addressed by runtime identifier

- **WHEN** an owner builds with a runtime identifier for a project whose
  toolchain addresses a target by runtime identifier
- **THEN** the build runs with that identifier bound

#### Scenario: A toolchain without a target mechanism

- **WHEN** an owner requests a target for a project whose toolchain has no
  target mechanism
- **THEN** the build is refused, and the refusal names the toolchain and the
  profile

#### Scenario: A partial target

- **WHEN** an owner passes an operating system without an architecture, or an
  architecture without an operating system
- **THEN** the build is refused, and the refusal names the missing half

#### Scenario: Two targets at once

- **WHEN** an owner passes a runtime identifier together with an operating
  system and an architecture
- **THEN** the build is refused

#### Scenario: A host build

- **WHEN** an owner builds without a target
- **THEN** the build runs for the host, no target is bound, and the outcome
  names the host

### Requirement: The target vocabulary is canonical

Forge SHALL accept one canonical spelling per operating system and per
architecture, SHALL accept the documented aliases, and SHALL refuse an unknown
value by name.

#### Scenario: An alias is accepted

- **WHEN** an owner passes a documented alias such as `macos` or `arm64`
- **THEN** the build resolves it to the canonical value

#### Scenario: An unknown value is refused

- **WHEN** an owner passes an operating system or an architecture Forge does
  not know
- **THEN** the build is refused, and the refusal names the value and the
  accepted values

### Requirement: The build reports only the target it used

Forge SHALL report the target the build actually addressed, in the form the
toolchain reads, and SHALL NOT report a target the build did not use.

#### Scenario: The reported target

- **WHEN** the build resolves a target
- **THEN** the outcome names the resolved target and the form it was resolved
  in

#### Scenario: The host is reported as the host

- **WHEN** the build runs for the host
- **THEN** the outcome names the host and no target is claimed
