## ADDED Requirements

### Requirement: The platform pins one reproducible .NET 10 toolchain

The platform repository MUST pin SDK `10.0.400`, reject prerelease SDKs, and
use a documented patch-only roll-forward policy for all repository-owned
MSBuild projects. SDK 8 MUST NOT remain a supported or selectable baseline.

#### Scenario: Repository uses the declared SDK

- **WHEN** `dotnet --version` is executed from the platform repository
- **THEN** it reports `10.0.400` or the exact allowed patch roll-forward of
  that feature band

#### Scenario: Prerelease SDK is installed

- **WHEN** a prerelease SDK is also available on the machine
- **THEN** repository resolution still selects the declared stable SDK

### Requirement: All platform-owned projects target .NET 10

All production, adapter, testing, test, sample, tool, and template projects
owned by this repository MUST target `net10.0`; inherited defaults and project
selection conditions MUST use the same target.

#### Scenario: Production solution is inspected

- **WHEN** the platform solution project graph is evaluated
- **THEN** every included project resolves `TargetFramework` to `net10.0`

#### Scenario: A generated application is created

- **WHEN** the platform application template is installed and instantiated
- **THEN** the generated application and its tests target `net10.0`

#### Scenario: Legacy target remains in a source-controlled project

- **WHEN** the final baseline scan searches source-controlled MSBuild,
  template, sample, and fixture metadata
- **THEN** any remaining `net8.0` target is reported as a verification failure

#### Scenario: SDK 8 remains selectable

- **WHEN** source-controlled `global.json`, CI setup, tool manifests, or
  current technical documentation is scanned
- **THEN** any SDK 8 pin or current instruction to use SDK 8 is reported as a
  verification failure, except explicitly labeled historical evidence

### Requirement: Microsoft package generations match the .NET 10 target

Microsoft ASP.NET Core, EF Core, and applicable Microsoft.Extensions package
references used by the platform MUST resolve to compatible 10.x versions from
the configured package source, while unrelated third-party packages remain
unchanged unless a compatibility failure justifies their update.

#### Scenario: Central package restore succeeds

- **WHEN** the solution is restored from the configured offline-compatible
  sources
- **THEN** all Microsoft framework package assets resolve without 8.x runtime
  package dependencies

#### Scenario: A third-party package is still version 8.x

- **WHEN** a non-Microsoft package has an 8.x version
- **THEN** it is not changed solely by the baseline migration and its
  compatibility decision is recorded if implementation updates it

### Requirement: Derived platform artifacts describe the .NET 10 baseline

The package manifest, consumer bootstrap, sample matrix, template metadata,
documentation, and verification scripts MUST agree with the source project
targets and package generation. Current technical documentation MUST describe
SDK 10 and .NET 10 consistently with the code.

#### Scenario: Package manifest is checked

- **WHEN** the package-manifest generator check runs
- **THEN** it passes and reports `net10.0` for every applicable platform package

#### Scenario: Consumer bootstrap evaluates a compatible project

- **WHEN** a generated `net10.0` consumer is evaluated with the platform
  checkout available
- **THEN** the expected local platform reference is selected and the opt-out
  behavior remains explicit and testable

#### Scenario: Documentation claims the old baseline

- **WHEN** source-controlled current documentation is scanned for the platform
  baseline
- **THEN** it identifies .NET 10, while historical handoff evidence remains
  unchanged and is not rewritten as current verification

#### Scenario: Current documentation retains an SDK 8 instruction

- **WHEN** a user follows a current README, runbook, architecture guide, or
  code comment describing the supported SDK or target
- **THEN** the instruction identifies SDK 10/.NET 10 and does not direct the
  user to install or select SDK 8

### Requirement: The baseline is not complete without full verification

The change MUST pass restore, serial build, test, pack, architecture,
public-API, manifest, formatting, and strict OpenSpec checks, or record an
explicit `BLOCKED` result with the exact command, failure, affected boundary,
and next action.

#### Scenario: All mandatory gates pass

- **WHEN** the complete repository gate runs under SDK `10.0.400`
- **THEN** all required checks pass and their evidence identifies the SDK,
  target framework, package source, and project scope

#### Scenario: Offline package is unavailable

- **WHEN** restore cannot find a required .NET 10 package in the available
  local sources
- **THEN** verification is marked `BLOCKED` and the missing package/source is
  reported; the change is not archived or called complete

#### Scenario: SDK 10 packing regression occurs

- **WHEN** SDK 10 build/tests pass but a `dotnet pack` gate fails
- **THEN** the failure remains visible as a packaging blocker and no workaround
  weakens warnings, skips pack, or claims package readiness

### Requirement: Downstream consumers receive an explicit migration contract

The completed change MUST document the required order and acceptance evidence
for workspace-level and repository-local C# consumer migrations without editing
those consumer repositories.

#### Scenario: Consumer migration begins after platform verification

- **WHEN** a consumer migration is selected
- **THEN** it can identify the required SDK pin, `net10.0` target, Microsoft
  package generation, runtime/tooling updates, and repository-native tests

#### Scenario: Consumer has application-owned migrations

- **WHEN** a consumer owns EF migrations, provider configuration, or runtime
  images
- **THEN** those changes remain in the consumer's own migration package and
  are not fabricated as evidence by the shared platform package
