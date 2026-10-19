# Build, test, pack, and validate

The repository uses standard .NET CLI commands. All commands run from the repository root unless noted.

## Restore

```bash
dotnet restore Platform.sln
```

The repository defaults to the Huawei Cloud NuGet mirror through
`NuGet.config`. Set `NUGET_SOURCE` when using an approved internal feed or the
official NuGet source:

```bash
NUGET_SOURCE=https://api.nuget.org/v3/index.json dotnet restore Platform.sln
```

The repository pins SDK `10.0.400` (`rollForward: latestPatch`,
`allowPrerelease: false`) in `global.json`; the supported baseline is
.NET 10. SDK 8 and `net8.0` are no longer selectable. See
[`docs/dotnet10-migration-contract.md`](dotnet10-migration-contract.md)
for the downstream migration contract and
[`docs/workspace-consumer-bootstrap.md`](workspace-consumer-bootstrap.md)
for the workspace-side bootstrap mechanics.

Package versions are defined centrally in `Directory.Packages.props`; project files declare `<PackageReference Include="..." />` without `Version` attributes.

## Build

```bash
dotnet build Platform.sln -c Release
```

Build defaults are centralized in `Directory.Build.props`:

- `Nullable=enable`, `ImplicitUsings=enable`, `LangVersion=latest`
- `Deterministic=true`, `TreatWarningsAsErrors=true`
- `GenerateDocumentationFile=true`
- `AnalysisLevel=latest-recommended`

Test projects override these via `tests/Directory.Build.props` to disable packaging and documentation generation. The solution currently contains 75 source projects (including the provider-neutral AI packages, the gap-audit additions, and the .NET 10 baseline).

## Test

```bash
dotnet test Platform.sln -c Release --nologo
```

Focused test projects live in `tests/`. `Platform.Architecture.Tests` enforces dependency direction (no production project may reference test projects, ASP.NET Core, EF Core, or Stripe; `Platform.Core` and `Platform.Billing.Contracts` must not reference any other project; `Platform.AspNetCore` may reference only `Platform.Core`).

## Pack

```bash
dotnet pack Platform.sln -c Release --no-build --nologo
```

Packages are written to each project's `bin/Release/` directory. Each package embeds the repository `README.md` via `PackageReadmeFile` and shares the repository `VersionPrefix` (`Directory.Build.props`) but may be released independently later by overriding `Version` per project.

Test projects set `IsPackable=false` and are not packed. The solution pack includes the testing-support projects under `src/` only when their project metadata allows packaging (`Platform.Testing`, `Platform.Testing.AspNetCore`, `Platform.Admin.Testing`, `Platform.Ai.Testing`, `Platform.Billing.Testing`, `Platform.Identity.Testing`, `Platform.Notifications.Testing`, `Platform.Quota.Testing`, `Platform.Tenant.Lifecycle.Testing`).

## Validate OpenSpec

```bash
openspec list
openspec validate --changes --strict --no-interactive
openspec validate --specs   --strict --no-interactive
```

Strict validation must pass before archiving a change. With no active changes, `openspec validate --specs --strict --no-interactive` validates the 46 synchronized generated specs.

## Consumer conformance

The repository ships a separate consumer-conformance fixture that consumes platform
packages from a local feed and is intentionally **not** part of `Platform.sln`. Run
`scripts/conformance.sh` to pack every platform project to a local feed, restore the
fixture, build it, and run the tests against the published artifacts:

```bash
./scripts/conformance.sh
```

The script records the exact failed command and the next action in
`tests/Platform.ConsumerConformance/.logs/` when a step fails so CI can distinguish a
source regression from an environment blocker (missing Docker, no local feed, no
network access). See [`docs/platform-consumer-conformance.md`](platform-consumer-conformance.md)
for the full layout and what the suite covers.

## Upgrade and rollback smoke test

When a previous package feed and a candidate package feed are available, verify
both adoption and rollback with:

```bash
./scripts/consumer-upgrade-rollback.sh \
  /absolute/path/to/previous-feed \
  /absolute/path/to/candidate-feed \
  0.2.0
```

The fixture is copied to a temporary directory, restored from the candidate
feed at `0.2.0`, tested, then restored from the previous feed at `0.1.0` and
tested again. The script exits non-zero on either failure and never modifies
the checked-in fixture.

## Lint working tree

```bash
git diff --check
```

`git diff --check` must report no whitespace errors before completion.
