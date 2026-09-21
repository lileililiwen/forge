# Proposal: Profile and release readiness evidence

## Why

The roadmap's v0.1 acceptance requires native generation/build/test evidence
for all five MVP profiles, while the repository currently has contract
fixtures and only partial native evidence. The repository also has no CI,
installable artifact or release-check workflow, so local build/test success is
not yet reproducible release evidence.

## What Changes

- Establish a versioned native-profile evidence matrix for the five v0.1
  profiles and the explicitly supported `react-web` profile.
- Add reproducible CI checks for formatting, build, tests, clippy, strict
  OpenSpec validation and generated-profile native commands where toolchains
  are available.
- Define a minimal distribution/install artifact and a release checklist that
  distinguishes unavailable toolchains from passing native evidence.

## BFS Impact Map

- **Capabilities and flows:** `forge new`, native verification, doctor evidence,
  release readiness and distribution of the Forge binary.
- **Modules, contracts and persistence:** profile descriptors, generator
  fixtures, native reports, release metadata and CI/package configuration.
- **Callers:** CLI and release checks; generated projects remain independent of
  Forge.
- **Dependencies:** `runtime-hardening-and-test-isolation`; each profile's
  native toolchain; existing release/distribution contracts.
- **Failure and boundary behavior:** missing toolchain, failed native command,
  non-reproducible bytes, unsupported profile and partial matrix results.
- **Tests:** profile matrix, generated fixture smoke tests, package install/
  version smoke tests and CI-equivalent local commands.
- **Compatibility/security/privacy:** no implicit downloads or remote writes;
  package contents are bounded and reproducible; no credentials in CI logs.
- **Unaffected:** new product capabilities, provider integrations and portal
  framework.

## Capabilities

### New Capabilities

- `profile-and-release-readiness`: reproducible native evidence and release
  gates for the currently supported profile/catalog surface.

### Modified Capabilities

- `deterministic-project-generation`: native evidence is recorded per profile.
- `release-publishing`: release readiness consumes the evidence matrix.

## Non-goals

This package does not promote planned specialist profiles, add package-manager
features to generated projects, publish to a remote registry, or treat a
missing toolchain as a pass.
