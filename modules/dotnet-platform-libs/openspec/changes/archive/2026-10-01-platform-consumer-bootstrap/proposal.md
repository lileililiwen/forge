# Proposal: One consumer bootstrap for the platform

## Why

Consumers adopt the platform through inconsistent paths, and the documented
switch does not work:

- `build/Platform.Consumer.props` injects only a `ProjectReference` to
  `Platform.Core`, and only when `TargetFramework` is exactly `net10.0`.
- `PlatformAsSource` is computed in the workspace `Directory.Build.props` and
  documented as the source/package switch, but `Platform.Consumer.props` never
  reads it, so the Maven-style source/package model is documentation only.
- The workspace root default target is `net8.0` while the platform is
  `net10.0`-only, so a default-target consumer silently receives nothing;
  `mewo` (a consumer) targets `net8.0`.
- Consumer defaults (`Nullable`, language version, analyzers, warnings policy,
  central package management) are not propagated; each consumer re-derives them
  and two consumers hand-copy them.
- No `dotnet nuget push` exists anywhere: the GitHub Packages feed is declared
  in the workspace `nuget.config` but never published to.
- Seven repositories adopt via `PackageReference` across four different feed
  paths with hand-written pack scripts; seven more are documented starting
  points.

## What Changes

- Make the source/package switch real: source → `ProjectReference`, package →
  `PackageReference`, with one documented entry point.
- Propagate consumer defaults (nullability, language version, analyzers,
  warnings policy) to platform consumers with an explicit opt-out.
- Emit a named diagnostic when a consumer targets an unsupported framework,
  instead of silently injecting nothing.
- Establish one version source and one publish path that pushes to the declared
  feed.
- Extend the existing consumer conformance to prove the bootstrap path from the
  published feed.

## Public API impact

**None.** This change touches MSBuild consumer hook-up, release tooling and
documentation only. No `Platform.*` public type, method, package identity or
package version line changes, and no shipped assembly's surface changes.

## BFS Impact Map

- **Capabilities:** consumer bootstrap, consumer defaults, release/publish.
- **Consumers:** the seven `PackageReference` adopters and the seven documented
  starting points; `for` no application behavior change.
- **Configuration:** `build/Platform.Consumer.props` (+ a `.targets` only if
  needed), the repository `Directory.Build.props`/`Directory.Packages.props`,
  `global.json`, and the release workflow.
- **Failure:** unsupported target framework, missing checkout with
  `PlatformAsSource=true`, and missing publish credentials must each fail with a
  named diagnostic.
- **Compatibility:** consumers that already pin versions and opt out keep
  working; the existing `platform-consumer-adoption` conformance is extended,
  not replaced.

## Capabilities

- `platform-consumer-bootstrap`.

## Non-goals

- No edits to any consumer repository; migration is documented, not performed.
- No new platform package, no public API change, no `Platform.Core` change.
- No replacement of the existing conformance workflow, only its extension.
- No deployment or environment provisioning.
- No change to the .NET 10 baseline itself (`dotnet10-platform-baseline`).

## Package Boundary and Split Assessment

**Single outcome:** a consumer can adopt the platform through one documented
path that works in both source and package modes.

**Included:** the MSBuild bootstrap files, the consumer-defaults propagation, the
unsupported-target diagnostic, the version/publish path, the conformance fixture
extension, and documentation.

**Excluded:** consumer repositories, the platform's public APIs, the .NET 10
target baseline (shipped), and the contract-source wiring
(`contracts-consumer-parity` in `platform-contracts`).

**Split signals considered:** bootstrap, defaults and publish share one owner,
one lifecycle (the consumer build), and one oracle (a fixture restores, builds
and tests in both modes). Separating publish from bootstrap would leave a
bootstrap that cannot be exercised by a real consumer.

**Dependencies:** `dotnet10-platform-baseline` (shipped, pins `net10.0`) and
`platform-consumer-adoption` (shipped, defines packed-artifact conformance).
Blocks consumer-side `net8.0` migration.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| this repository | `openspec/specs/dotnet10-platform-baseline/spec.md` | pins SDK 10 / `net10.0`; mentions consumer bootstrap selection and opt-out | states the behaviour but the hook does not read `PlatformAsSource` | dotnet-platform-libs | **extend shared owner** |
| this repository | `openspec/specs/platform-consumer-adoption/spec.md` | packed-artifact conformance, pinned versions, upgrade/rollback, boundaries | covers conformance, not the bootstrap switch itself | dotnet-platform-libs | **adopt** (extend the fixture) |
| this repository | `templates/`, `tools/Platform.Adoption.Tool` | starter template and adoption doctor | assume one path | dotnet-platform-libs | **adopt** |
| workspace `Directory.Build.props` / `nuget.config` | `PlatformAsSource`, `PlatformPackageVersion`, feed definitions | the workspace-level switch and feeds | lives outside this repository; document the contract, do not edit it here | workspace-governance | **adapt through a generic adapter** |
| platform-contracts | `openspec/specs/contract-foundation/spec.md` | contract schemas | out of scope | platform-contracts | **not applicable** |

## Evidence boundary

This change can prove that the bootstrap selects the right reference mode, that
defaults propagate, and that a fixture restores, builds and tests from the
published feed. It cannot prove that any consumer application runs in
production, and it does not migrate consumers.
