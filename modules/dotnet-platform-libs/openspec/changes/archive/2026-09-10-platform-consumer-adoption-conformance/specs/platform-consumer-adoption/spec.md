## ADDED Requirements

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
