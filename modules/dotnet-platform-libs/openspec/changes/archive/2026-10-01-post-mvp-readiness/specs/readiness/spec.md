# readiness Specification

## Purpose

Give consumers one authoritative front door to the platform: a complete
per-package matrix, a versioning and consumer guide, minimal usage examples, an
explicit license, and an adoption entry point — without changing any public API
or package.

## ADDED Requirements

### Requirement: The README SHALL present an authoritative per-package matrix

`README.md` MUST list every shipped package under `src/` with its kind
(production, optional adapter, or test-only), dependency direction, and purpose,
grouped by concern, and MUST reference `docs/packages.md` for the detailed
surface. The matrix MUST NOT name a package that does not exist.

#### Scenario: A consumer finds a package

- **WHEN** a consumer looks up a package in the README matrix
- **THEN** the row names its kind, what it depends on, and its purpose, and
  links to the surface reference

#### Scenario: Matrix is incomplete or wrong (negative)

- **WHEN** a shipped package is absent from the matrix, or a listed package does
  not exist
- **THEN** the readiness requirement is not met

### Requirement: The README SHALL summarize versioning and consumer adoption

`README.md` MUST state the single version source, the SemVer policy, the
`net10.0` baseline, source-vs-package mode, central package management, and
upgrade/rollback, with links to the governing documents.

#### Scenario: A consumer plans adoption

- **WHEN** a consumer reads the versioning/consumer section
- **THEN** the version source, compatibility rules, reference modes, and
  upgrade/rollback are named with links

#### Scenario: Guide contradicts a governing rule (negative)

- **WHEN** the section states a version or compatibility rule that contradicts
  `docs/release-governance.md` or the baseline
- **THEN** the requirement is not met

### Requirement: Usage examples SHALL be provided and verifiable

The README MUST provide a minimal usage example for each major concern (at least
Core, Web composition, Jobs, Mailing, Eventing, Caching, Storage, Quota,
Identity, and Tenant lifecycle), and each example MUST be consistent with an
existing sample, template, or documented package surface.

#### Scenario: A consumer copies an example

- **WHEN** a consumer copies a README example
- **THEN** the referenced types and registration method match the documented
  surface and the corresponding sample

#### Scenario: Example contradicts the surface (negative)

- **WHEN** an example names a type or method that does not exist, or contradicts
  `docs/packages.md`
- **THEN** the requirement is not met

### Requirement: Adoption SHALL have a single documented entry point

The README MUST point to the platform adoption guide, the adoption diagnostics
tool, and the sample matrix so a consumer starts from one place.

#### Scenario: Adoption start point

- **WHEN** a consumer wants to adopt the platform
- **THEN** the README names the adoption guide, the tool, and the sample matrix

#### Scenario: Adoption paths diverge (negative)

- **WHEN** two conflicting adoption entry points are documented with no primary
- **THEN** the requirement is not met

### Requirement: The repository SHALL declare a license

A `LICENSE` file MUST exist, `README.md` MUST reference the declared terms, and
the declaration MUST match `PackageLicenseExpression` in `Directory.Build.props`.

#### Scenario: License declared and reconciled

- **WHEN** the repository root, README, and package metadata are inspected
- **THEN** a `LICENSE` file exists and its terms match the package license
  expression

#### Scenario: License absent or contradictory (negative)

- **WHEN** the `LICENSE` file is missing, or states terms different from the
  package metadata
- **THEN** the requirement is not met

### Requirement: Screenshots SHALL be recorded NOT_APPLICABLE with justification

Because the repository ships no user-facing UI, the change MUST record
screenshots as `NOT_APPLICABLE` with the reason, and readiness MUST NOT fail for
their absence.

#### Scenario: No UI to capture

- **WHEN** readiness is assessed
- **THEN** screenshots are recorded `NOT_APPLICABLE` and do not block readiness

#### Scenario: A sample UI ships without screenshots (negative)

- **WHEN** a user-facing sample UI is shipped and the no-UI justification no
  longer holds
- **THEN** the `NOT_APPLICABLE` record MUST be revisited rather than left stale