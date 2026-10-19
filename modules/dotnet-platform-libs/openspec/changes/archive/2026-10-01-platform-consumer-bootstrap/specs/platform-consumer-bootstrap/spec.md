# platform-consumer-bootstrap Specification

## Purpose

Give the platform one consumer bootstrap path that actually works in both source
and package mode, propagates the consumer defaults, and fails loudly rather than
silently injecting nothing.

## ADDED Requirements

### Requirement: The source/package switch SHALL be honoured

The consumer bootstrap MUST select a `ProjectReference` to the platform source
when source mode is enabled and a local checkout exists, and a
`PackageReference` at the pinned version otherwise. The switch MUST be read by
the bootstrap itself, not merely declared in documentation.

#### Scenario: Source mode with a checkout

- **WHEN** a consumer enables source mode and the platform checkout exists
- **THEN** the consumer resolves the platform by project reference

#### Scenario: Source mode without a checkout

- **WHEN** a consumer enables source mode and no checkout exists
- **THEN** evaluation fails with a diagnostic naming the switch and the missing
  path, and does not silently fall back to a package

#### Scenario: Package mode

- **WHEN** source mode is disabled and the feed is reachable
- **THEN** the consumer resolves the platform by package reference at the pinned
  version

### Requirement: Consumer defaults SHALL propagate with an opt-out

When a consumer has not opted out, the bootstrap MUST apply the shared defaults
for nullability, language version, analyzer level, warnings policy, and central
package management, and each default MUST be overridable by the consumer.

#### Scenario: Defaults applied

- **WHEN** a consumer does not opt out
- **THEN** nullable, language version, analyzers and warnings policy resolve to
  the platform defaults

#### Scenario: Consumer opts out

- **WHEN** a consumer sets the opt-out property
- **THEN** no reference and no default is applied, and no diagnostic is emitted

#### Scenario: Consumer overrides a default

- **WHEN** a consumer sets one of the defaulted properties itself
- **THEN** the consumer's value wins

### Requirement: An unsupported target framework SHALL be diagnosed

A consumer whose target framework is not supported by the platform MUST receive
a named diagnostic identifying its target and the supported target, and MUST NOT
silently receive no reference.

#### Scenario: Unsupported target

- **WHEN** a consumer targets a framework the platform does not publish
- **THEN** the build fails with a diagnostic naming both frameworks

### Requirement: There SHALL be one version source and one publish path

The platform version used by consumers MUST come from a single declared source,
and a release MUST publish the packed packages to the declared feed rather than
only uploading build artifacts.

#### Scenario: Version declared once

- **WHEN** the platform version is inspected
- **THEN** it is declared in one place and consumers reference it

#### Scenario: Release publishes

- **WHEN** the release workflow runs with credentials
- **THEN** the packed packages are pushed to the declared feed and the release
  records the publication

#### Scenario: Credentials absent

- **WHEN** the release workflow runs without publish credentials
- **THEN** it fails loudly instead of skipping publication

### Requirement: Conformance SHALL cover the bootstrap path

The consumer conformance workflow MUST exercise both source and package modes,
the unsupported-target case, and the opt-out case, in addition to the existing
packed-artifact and pinned-version checks.

#### Scenario: Both modes verified

- **WHEN** conformance runs
- **THEN** a fixture consumer restores, builds and tests in source mode and in
  package mode

#### Scenario: Opt-out verified

- **WHEN** conformance evaluates an opted-out fixture
- **THEN** the platform is absent from its resolved references
