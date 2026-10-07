# platform-consumer-adoption Specification

## Purpose

The platform will be consumed by 26+ repositories with different SDK versions, persistence models, and deployment environments. Current conformance coverage exists inside the platform repository, but consumers need a repeatable pinned-package adoption, upgrade, rollback, and architecture-verification route.
## Requirements
### Requirement: Consumer conformance SHALL use packed artifacts

The conformance workflow SHALL restore platform packages from a local or private NuGet feed and SHALL not use cross-repository project references to the platform source.

#### Scenario: Candidate package adoption
- **WHEN** a candidate package is packed into the feed
- **THEN** the consumer fixture restores that package version and verifies its public registration and behavior

### Requirement: Consumers SHALL pin platform versions

Adoption documentation and fixtures SHALL use exact platform package versions and SHALL identify transitive dependency audit results.

#### Scenario: Floating version detected
- **WHEN** a consumer fixture declares a floating platform version
- **THEN** conformance fails with the package and remediation required

### Requirement: Upgrade and rollback SHALL be tested

The workflow SHALL verify upgrading from the previous approved package version to the candidate and restoring the previous version after a failed candidate validation.

#### Scenario: Candidate regression
- **WHEN** the candidate fails a consumer contract test
- **THEN** the workflow restores the prior version and verifies the consumer remains buildable

### Requirement: Consumer boundaries SHALL be enforced

Conformance SHALL verify that production consumers do not reference platform testing packages and that application-owned entities, migrations, plans, and provider identifiers remain outside platform packages.

#### Scenario: Forbidden production dependency
- **WHEN** a production consumer references `Platform.Testing`
- **THEN** architecture conformance fails with the project and dependency path

### Requirement: Shared MSBuild policy SHALL be obtainable from the platform's packages

The platform's consumer adoption documentation SHALL identify the packed policy
package as the supported way for a consumer repository to obtain shared MSBuild
policy, so that a consumer does not obtain build policy from an unversioned file
found by a directory walk outside version control. The route MUST satisfy the
same packed-artifact, exact-version, and upgrade/rollback requirements that
govern the other platform packages, and MUST be documented as distinct from the
import-by-path consumer bootstrap route.

#### Scenario: Documented route is discoverable

- **WHEN** a consumer repository is adopting the platform
- **THEN** the adoption documentation states that shared MSBuild policy is
  obtained by referencing the policy package at an exact version

#### Scenario: Not confused with the import-by-path route

- **WHEN** a reader follows the adoption documentation to the policy package
- **THEN** they can distinguish it from the consumer bootstrap route, which
  requires importing a file by path and an explicit opt-in, and each document
  links to the other

#### Scenario: Policy is not obtained from an unowned file

- **WHEN** a consumer repository follows the documented adoption route
- **THEN** its shared MSBuild policy comes from a versioned package it declares,
  and not from a file outside version control that the build discovers
  implicitly

### Requirement: New and legacy consumers SHALL share one portable adoption shape

The platform SHALL provide a canonical consumer configuration and an
idempotent legacy migration that retain exact package versions, allow an
execution-time version override, and contain no committed local feed path.

#### Scenario: Legacy migration applies safely

- **WHEN** a legacy consumer has one exact version across its `Platform.*` references
- **THEN** apply mode writes the canonical feed mapping and single defaulted version property, then its check mode passes

#### Scenario: Legacy migration detects ambiguity

- **WHEN** a legacy consumer carries multiple, floating or unresolved `Platform.*` versions
- **THEN** migration exits non-zero and writes nothing

#### Scenario: Already conforming consumer

- **WHEN** check or apply mode examines an already conforming consumer
- **THEN** it succeeds without changing bytes

### Requirement: Agent guidance SHALL keep local sources user-scoped

The platform adoption skill SHALL direct agents to configure local feeds only
in the current user's NuGet configuration and to validate clean consumers
without that local configuration.

#### Scenario: Agent tests unpublished packages

- **WHEN** an agent needs an unpublished platform build
- **THEN** it packs a unique prerelease, uses the user-level local source and passes the version as an MSBuild property without editing tracked consumer configuration

#### Scenario: Agent claims portability

- **WHEN** an agent reports a consumer as portable
- **THEN** it has verified restore/build with an isolated package cache and without the user-level local source
