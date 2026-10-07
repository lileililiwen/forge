# platform-consumer-bootstrap Specification

## Purpose

Consumers adopt the platform through inconsistent paths, and the documented switch does not work: - `build/Platform.Consumer.props` injects only a `ProjectReference` to `Platform.Core`, and only when `TargetFramework` is exactly `net10.0`. - `PlatformAsSource` is computed in the workspace `Directory.Build.props` and documented as the source/package switch, but `Platform.Consumer.props` never reads it, so the Maven-style source/package model is documentation only. - The workspace root default target is `net8.0` while the platform is `net10.0`-only, so a default-target consumer silently receives nothing; `mewo` (a consumer) targets `net8.0`. - Consumer defaults (`Nullable`, language version, analyzers, warnings policy, central package management) are not propagated; each consumer re-derives them and two consumers hand-copy them. - No `dotnet nuget push` exists anywhere: the GitHub Packages feed is declared in the workspace `nuget.config` but never published to. - Seven repositories adopt via `PackageReference` across four different feed paths with hand-written pack scripts; seven more are documented starting points.
## Requirements
### Requirement: The source/package switch SHALL be honoured

New and migrated consumers MUST use `PackageReference` for `Platform.*` and
MUST NOT resolve platform projects through a sibling checkout. The retained
legacy bootstrap MAY continue to honour its source/package switch only as a
temporary compatibility path for consumers that have not yet migrated.

#### Scenario: New or migrated consumer

- **WHEN** a new or migrated consumer references a platform library
- **THEN** it resolves an exact package version through NuGet without a platform source path

#### Scenario: Unpublished local package

- **WHEN** a developer tests an unpublished platform change
- **THEN** the consumer still uses `PackageReference` and resolves a unique prerelease from a user-level local source

#### Scenario: Legacy bootstrap remains temporarily usable

- **WHEN** an unmigrated consumer still enables the retained bootstrap
- **THEN** its prior switch behavior remains available until that consumer is migrated

#### Scenario: Source mode with a checkout

- **WHEN** an unmigrated consumer enables legacy source mode and the checkout exists
- **THEN** the compatibility bootstrap resolves the project reference and emits deprecation guidance

#### Scenario: Source mode without a checkout

- **WHEN** an unmigrated consumer enables legacy source mode and the checkout is absent
- **THEN** evaluation fails rather than silently selecting a different dependency

#### Scenario: Package mode

- **WHEN** legacy source mode is disabled
- **THEN** the compatibility bootstrap resolves the exact package version

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
and a release MUST publish immutable packages to the anonymous static NuGet
feed through the GitHub Pages deployment workflow. Consumer restore MUST NOT
require a GitHub token or package-registry credential.

#### Scenario: Version declared once

- **WHEN** the platform version is inspected
- **THEN** it is declared in one place and consumers reference it

#### Scenario: Release publishes

- **WHEN** the release workflow runs with repository Pages permissions
- **THEN** it updates the persistent static feed, validates it, records publication evidence and deploys Pages

#### Scenario: Consumer restores anonymously

- **WHEN** a clean consumer restores a published `Platform.*` version
- **THEN** restore uses the public HTTPS endpoint without a token

#### Scenario: Credentials absent

- **WHEN** a consumer has no GitHub or package-registry credentials
- **THEN** restore from the public static feed remains supported

### Requirement: Conformance SHALL cover the bootstrap path

Consumer conformance MUST exercise package restore from a generated local feed,
the canonical source mapping, exact version override, and absence of platform
project references. The legacy source-mode fixture MAY remain while the
compatibility bootstrap exists but is not portability evidence.

#### Scenario: Package-only path verified

- **WHEN** conformance runs
- **THEN** a fixture consumer restores, builds and tests exclusively through `PackageReference`

#### Scenario: Repository path independence verified

- **WHEN** the portable consumer fixture is inspected
- **THEN** it contains no sibling platform checkout or absolute local feed path

#### Scenario: Both modes verified

- **WHEN** compatibility conformance runs while the legacy bootstrap remains
- **THEN** its source and package fixtures pass, but only the package fixture counts as portability evidence

#### Scenario: Opt-out verified

- **WHEN** compatibility conformance evaluates an opted-out legacy fixture
- **THEN** the platform remains absent from its resolved references
