# platform-build-policy Specification

## Purpose

Shared MSBuild policy reaches consuming .NET projects through an unowned file
that MSBuild finds by walking up the directory tree:
`/home/paul/code/Directory.Build.props` lives outside any Git repository, so no
commit records it, no review sees it, and no history explains it. Forge emits no
`Directory.Build.props`, so new scaffolds inherit it too. This capability makes
the policy a versioned, centrally owned, packed artifact that a project opts into
with a single pinned `PackageReference`.

## ADDED Requirements

### Requirement: Shared MSBuild policy SHALL ship as a packed props package

The platform SHALL provide a package that carries the shared MSBuild policy in a
`buildTransitive/` props file, so that a project with a `PackageReference` to it
receives the policy automatically, at a known version, with no per-repository
copy and no import.

#### Scenario: A reference receives the policy without an import

- **WHEN** a project adds a `PackageReference` to the policy package at an exact
  version
- **THEN** the package's props file is applied to that project without the
  project importing any file by path, and the version applied is recorded in the
  project's dependency declaration

#### Scenario: A project without the reference is unaffected

- **WHEN** a project does not reference the policy package
- **THEN** it receives none of the policy, and the props file is not evaluated

#### Scenario: The package is versioned and packed like any platform package

- **WHEN** the platform packs its solution
- **THEN** the policy package is produced as a `.nupkg` at the platform version
  and is present in the same local feed the existing consumer conformance uses

### Requirement: The policy package SHALL assert policy, not project identity

The policy package SHALL assert warnings policy, code-style enforcement,
deterministic output, the justified `NoWarn` list, central package management
including transitive pinning, and the continuous-integration build flag. It
SHALL NOT assert `TargetFramework` or `RootNamespace`, which belong to the
consuming project.

#### Scenario: Policy properties resolve

- **WHEN** a project references the policy package
- **THEN** the warnings, code-style, determinism, suppression, central package
  management and CI properties resolve to the recorded platform policy

#### Scenario: The package does not impose a target framework

- **WHEN** a project that targets a framework other than the platform default
  references the policy package
- **THEN** the package does not change that project's target framework

#### Scenario: Every suppression carries a justification

- **WHEN** the policy package's `NoWarn` list is reviewed
- **THEN** each entry is accompanied by a one-line justification that a reviewer
  can re-evaluate, and an unjustified entry is rejected

### Requirement: Policy ownership SHALL be single

Each MSBuild property SHALL be asserted by exactly one owner. A property already
asserted by the platform's existing consumer bootstrap SHALL NOT also be asserted
by the policy package, and the two SHALL NOT be able to disagree.

#### Scenario: No property is asserted twice

- **WHEN** the policy package's props and the existing consumer bootstrap are both
  in scope for a project
- **THEN** no property is asserted by both, so the resolved value does not depend
  on which file is imported first

#### Scenario: The consumer's own value always wins

- **WHEN** a project sets a property that the policy package would otherwise
  assert
- **THEN** the project's value is used and the package does not override it

### Requirement: The policy package SHALL be obtainable through the existing adoption path

`docs/platform-consumer-adoption.md` SHALL describe the policy package as the
supported way for a consumer repository to obtain shared MSBuild policy, subject
to the same exact-version pinning, packed-artifact conformance, and
upgrade/rollback rehearsal that govern every other platform package.

#### Scenario: Pinned exactly

- **WHEN** a consumer declares a reference to the policy package
- **THEN** the version is an exact version, and a floating range, wildcard or
  version range is rejected by consumer conformance

#### Scenario: Restored from a packed artifact

- **WHEN** consumer conformance runs
- **THEN** the policy package is restored as a packed artifact from the feed, not
  resolved through a cross-repository project reference

#### Scenario: Upgrade and rollback are rehearsed

- **WHEN** the upgrade/rollback script runs for the policy package
- **THEN** the candidate version is verified, then the previous version is
  restored and re-verified

#### Scenario: Documented in both directions

- **WHEN** a reader starts from either the consumer adoption document or the
  consumer bootstrap document
- **THEN** each points to the other and states which route it describes

### Requirement: Conformance SHALL prove the policy is applied, not merely restored

Consumer conformance MUST assert the *resolved value* of each property the
policy package owns. A successful restore and build of a build-only package is
not sufficient evidence, because a build-only package restores and builds
successfully whether or not its props are honoured.

#### Scenario: Resolved value asserted

- **WHEN** the conformance fixture builds with the policy package referenced
- **THEN** it asserts the resolved value of each owned property and fails if any
  differs from the recorded policy

#### Scenario: Override asserted

- **WHEN** a second fixture sets one owned property itself
- **THEN** the fixture asserts the project's value survives the policy package

#### Scenario: Build-only shape asserted

- **WHEN** the policy package is packed
- **THEN** it contains the `buildTransitive` props file, and conformance fails if
  the props file is absent or not imported
