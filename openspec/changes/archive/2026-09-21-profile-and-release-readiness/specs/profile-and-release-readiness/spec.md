# profile-and-release-readiness Specification

## Purpose

Provide reproducible native evidence for supported generated profiles and a
CI/artifact gate that can distinguish release readiness from local compilation.

## ADDED Requirements

### Requirement: Supported profile native evidence

Forge SHALL maintain a reproducible native evidence row for every supported
profile, including the five v0.1 profiles and `react-web`.

#### Scenario: Native evidence passes

- **WHEN** a generated fixture runs its declared native build and test commands without Forge
- **THEN** the row records toolchain versions, commands, source identity and a passed result

#### Scenario: Native command fails

- **WHEN** generation succeeds but a declared native build or test command fails
- **THEN** the profile is not reported as verified and the failure identifies the profile and command

#### Scenario: Toolchain is unavailable

- **WHEN** the host or CI runner lacks a required toolchain
- **THEN** the row is unverified with the missing prerequisite and cannot satisfy a release gate

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
