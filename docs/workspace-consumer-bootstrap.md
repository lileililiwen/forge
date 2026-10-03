# Workspace consumer bootstrap

> **This document describes the import-by-path route only.** It requires
> importing a file by path and opting in with `PlatformConsumerBootstrap=true`.
> For the restore-driven route — a versioned package reference that needs no
> import and no opt-in — see
> [`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md), which
> documents `Platform.Build` as the supported way to obtain shared MSBuild
> policy. The two are complementary and must not be mixed for the same
> property: `build/Platform.Consumer.props` owns `Nullable`, `LangVersion`,
> `AnalysisLevel`, `TreatWarningsAsErrors` and `ManagePackageVersionsCentrally`,
> while `Platform.Build` owns the code-style, determinism, central-pinning and
> CI properties. Each property has exactly one owner, and
> `BuildPolicyConformanceTests` fails the build if either file starts claiming
> the other's.

The workspace uses .NET 10 as its default SDK and target framework for new
MSBuild projects.

The automatic hook is installed at the mixed-language workspace root:

- `/home/paul/code/global.json` pins the .NET SDK to `10.0.400`.
- `/home/paul/code/Directory.Build.props` supplies `net10.0` when a project
  has not selected a target framework, and declares the source/package
  switch (`PlatformAsSource`) and the pinned version (`PlatformPackageVersion`).
- `/home/paul/code/Directory.Build.targets` imports the platform consumer
  configuration after each project file has been evaluated.
- `build/Platform.Consumer.props` is the platform-owned consumer bootstrap.
  It honours the switch, propagates the consumer defaults, and emits a
  named diagnostic when the consumer targets a framework the platform does
  not publish.

Python, Rust, and other non-MSBuild projects ignore these files. The platform
repository is also excluded, so it does not reference itself.

## Contract

| Property | Default | Effect |
|---|---|---|
| `PlatformConsumerBootstrap` | unset | Opt-in switch. When `true`, the bootstrap is active. When unset or `false`, the bootstrap is a no-op (no reference, no defaults, no diagnostic). This is the guard that keeps the workspace's default `net8.0` projects silent. |
| `PlatformConsumerOptOut` | unset | Suppresses every default, every reference, and every diagnostic, even when the bootstrap is active. Honoured for back-compat with the original opt-out. |
| `PlatformAsSource` | auto | `true` when a local platform checkout exists, `false` otherwise. `true` injects a `ProjectReference`; `false` (or unset) injects a `PackageReference` at the pinned version. |
| `PlatformPackageVersion` | `0.1.0` | The version used when `PlatformAsSource` is not `true`. Unset value fails the build with a named diagnostic. |
| `PlatformConsumerSourceRoot` | auto | The path to the local platform checkout. Required when `PlatformAsSource=true`; a missing checkout fails the build with a named diagnostic. |

## Consumer states

| Consumer state | Result |
|---|---|
| `PlatformAsSource=true`, local checkout present | `ProjectReference` to `Platform.Core` |
| `PlatformAsSource=true`, checkout missing | named error at evaluation; no silent package fallback |
| `PlatformAsSource=false`, feed reachable | `PackageReference` to `Platform.Core` at the pinned version |
| `TargetFramework` not `net10.0` | named diagnostic naming the consumer's target and the supported target |
| `PlatformConsumerOptOut=true` | no injection, no defaults, no diagnostic |

## Consumer defaults

When the bootstrap is active and the consumer has not opted out, the
bootstrap applies the following defaults. Each default is overridable by
the consumer — the bootstrap only sets a property when the consumer has
not already set it.

| Default | Value |
|---|---|
| `Nullable` | `enable` |
| `LangVersion` | `latest` |
| `AnalysisLevel` | `latest-recommended` |
| `TreatWarningsAsErrors` | `true` |
| `ManagePackageVersionsCentrally` | `true` |

These five belong to this file and to nowhere else. Do not re-assert them in a
`Directory.Build.props` of your own and do not expect the `Platform.Build`
policy package to supply them: a property that two `== ''`-guarded files assert
resolves by import order, and the file that loses is silent about it. The
policy package covers a disjoint set of properties; see
[`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md).

### Central package management and `Platform.Core`

The bootstrap enables `ManagePackageVersionsCentrally=true` by default.
In package mode (`PlatformAsSource` not `true`), the bootstrap then
injects the `Platform.Core` reference in the form central package
management expects:

- When `ManagePackageVersionsCentrally=true` (the default), the
  bootstrap injects `<PackageReference Include="Platform.Core" />`
  without an inline version. The consumer must declare the version
  centrally in their `Directory.Packages.props`:

  ```xml
  <ItemGroup>
    <PackageVersion Include="Platform.Core" Version="$(PlatformPackageVersion)" />
  </ItemGroup>
  ```

- When `ManagePackageVersionsCentrally!=true`, the bootstrap injects
  `<PackageReference Include="Platform.Core" Version="$(PlatformPackageVersion)" />`
  with the version set inline.

In both cases the version comes from the consumer's `$(PlatformPackageVersion)`
property. Central package management is the recommended path because
it keeps every package version in a single auditable file.

## Opt out

A .NET project that must remain independent can set this property in its
project file:

```xml
<PropertyGroup>
  <PlatformConsumerOptOut>true</PlatformConsumerOptOut>
</PropertyGroup>
```

The opt-out is honoured even when the bootstrap is active. A consumer that
wants the bootstrap to do nothing at all can either skip the opt-in
(`PlatformConsumerBootstrap` unset) or set the opt-out.

## Adopt the platform without the workspace hook

A consumer that does not live under the workspace (e.g., a fresh clone
of a product repository) imports the bootstrap explicitly:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net10.0</TargetFramework>
    <PlatformConsumerBootstrap>true</PlatformConsumerBootstrap>
    <!-- Source mode -->
    <PlatformAsSource>true</PlatformAsSource>
    <PlatformConsumerSourceRoot>/path/to/dotnet-platform-libs</PlatformConsumerSourceRoot>
    <!-- Package mode -->
    <!-- (omit PlatformAsSource) -->
    <!-- <PlatformPackageVersion>0.1.0</PlatformPackageVersion> -->
  </PropertyGroup>
  <Import Project="/path/to/dotnet-platform-libs/build/Platform.Consumer.props" />
</Project>
```

For the .NET 10 baseline, the SDK pin, the Microsoft 10.x package
generation, and the per-consumer migration order, see
[`docs/dotnet10-migration-contract.md`](dotnet10-migration-contract.md).

## Release and publish

`scripts/conformance.sh` exercises both source and package modes plus the
unsupported-target and opt-out cases. The release workflow
(`.github/workflows/release.yml`) packs the platform and pushes the
resulting `.nupkg` files to the declared feed when `GITHUB_TOKEN` is
available; a release without publish credentials fails loudly rather than
silently skipping publication.

Run the bootstrap check from this repository:

```bash
bash scripts/test-workspace-bootstrap.sh
```

Run the full consumer conformance (pack + restore + build + test + bootstrap
fixtures) from this repository:

```bash
bash scripts/conformance.sh
```
