## ADDED Requirements

### Requirement: The .NET platform project is an independently runnable Forge module

Forge SHALL contain the complete tracked `dotnet-platform-libs` project as ordinary content under `modules/dotnet-platform-libs/`, retain its native project and package boundaries, and verify the import against the recorded source revision. The module SHALL remain independently buildable and testable without the source sibling checkout.

#### Scenario: Imported source and history are verified

- **WHEN** Forge verifies its imported .NET platform module
- **THEN** the check confirms the recorded source commit and all source-only commits are in Forge history, compares tracked paths, modes and blobs against that commit except declared monorepo adaptations, and fails on missing, extra or undeclared changed files

#### Scenario: Maintainer builds the module in isolation

- **WHEN** a maintainer runs the module's declared verification command from `modules/dotnet-platform-libs/`
- **THEN** the module-local .NET SDK pin, solution and source resolve within the Forge checkout and the command does not fall back to `/home/paul/code/dotnet-platform-libs`

#### Scenario: The module retains .NET and UI package identity

- **WHEN** the module's NuGet packages and UI feed artifacts are built from the imported source
- **THEN** package IDs, versions, target frameworks, licenses, npm package names, local feed layout and consumer-conformance behavior match the recorded source project

### Requirement: Forge runs module-scoped quality and explicit release workflows

Forge SHALL discover the imported module's quality workflow from the repository-root `.github/workflows/` directory and run its existing native quality commands when relevant module paths change. Package release SHALL remain an explicit, version-validated manual workflow and SHALL NOT be triggered by ordinary push or pull-request CI.

#### Scenario: A .NET module change runs quality checks

- **WHEN** a pull request or main push changes a file under `modules/dotnet-platform-libs/` or its root workflow
- **THEN** Forge schedules the module's build, test, API, audit and package-conformance checks with the module as their working directory

#### Scenario: CI cannot publish packages

- **WHEN** a .NET module pull request or main push runs its quality workflow
- **THEN** it performs no NuGet, npm or GitHub Pages write and does not invoke the release workflow

#### Scenario: NuGet release is manually requested

- **WHEN** an operator dispatches the module release workflow with a SemVer version
- **THEN** the workflow validates the version, runs all release preconditions, retains the static feed at Forge's Pages endpoint and reports packed-artifact and deployment evidence; any failed precondition prevents publication/deployment

### Requirement: In-tree governance resolves the .NET platform module portably

Forge-owned project and UI-conformance checks SHALL resolve `dotnet-platform-libs` from the Forge monorepo layout and SHALL NOT require an absolute sibling checkout path for source, token or project-registry reads.

#### Scenario: The module project record resolves in Forge

- **WHEN** Forge Governance reads the `dotnet-platform-libs` project declaration
- **THEN** the preserved project id resolves to `forge/modules/dotnet-platform-libs` and every referenced declaration exists in the Forge checkout

#### Scenario: UI token conformance runs without the origin checkout

- **WHEN** the in-tree UI-conformance script and its tests resolve platform token sources
- **THEN** they read `modules/dotnet-platform-libs/ui/` using module-relative paths and pass without `/home/paul/code/dotnet-platform-libs`
