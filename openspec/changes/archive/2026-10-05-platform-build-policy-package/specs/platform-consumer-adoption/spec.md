# platform-consumer-adoption Specification

## ADDED Requirements

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
