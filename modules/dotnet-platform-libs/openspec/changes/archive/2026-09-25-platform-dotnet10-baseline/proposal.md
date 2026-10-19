## Why

The shared platform currently targets `net8.0` and pins SDK `8.0.424`, while the
workspace is moving its C# projects to one reproducible .NET 10 baseline. A
verified shared-library baseline must be established before consumer projects
can migrate without mixing SDK, target-framework, Microsoft package, template,
and test-tool generations.

## What Changes

- **BREAKING**: Remove SDK 8 immediately. Pin this repository to SDK `10.0.400`
  with the standard `latestPatch` roll-forward policy and target all platform
  source, test, tool, sample, and template projects at `net10.0`.
- **BREAKING**: Remove `net8.0` immediately. Move centrally managed ASP.NET
  Core, EF Core, and applicable
  Microsoft.Extensions package references from 8.x to the compatible 10.x
  generation; retain third-party package versions unless their own .NET 10
  compatibility requires a separately justified update.
- Update the platform consumer bootstrap, sample matrix, package manifest,
  generated template inputs, documentation, and verification scripts so they
  consistently describe SDK 10 and `net10.0`; current technical documentation
  that says SDK 8 or .NET 8 is updated in the same change.
- Add explicit restore, build, test, pack, API, architecture, manifest, and
  offline-cache verification for the new baseline.
- Publish a downstream migration contract identifying the C# consumer changes
  required after this shared package is verified.

## Package Boundary and Split Assessment

This change owns only the reusable `dotnet-platform-libs` repository. Consumer
projects have different owners, package graphs, database migrations, runtime
images, and release gates, so their migrations are separate dependent changes.
The shared baseline is the smallest independently verifiable prerequisite: it
must produce usable .NET 10 packages before any consumer can adopt them.

The downstream package order is:

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `platform-dotnet10-baseline` | Shared platform packages build and pack for .NET 10 | `dotnet-platform-libs`, C# | SDK `10.0.400`, `net10.0`, Microsoft 10.x package generation | None | Full platform restore/build/test/pack and offline verification |
| `dotnet10-consumer-baseline` | Workspace C# SDK and target defaults use .NET 10 | `/home/paul/code` MSBuild boundary | Root `global.json`, `Directory.Build.props`, and consumer-local overrides | `platform-dotnet10-baseline` | Inventory has no targeted 8.x SDK/TFM declarations and representative projects build |
| `<consumer>-dotnet10-migration` | One application repository migrates its code, packages, runtime, and migrations | Each C# consumer repository, C# | Repository-local project/package/runtime contract | `dotnet10-consumer-baseline` and shared packages | Repository-native restore/build/test/runtime or migration checks |

The following is an inventory for separately authored follow-up proposals only;
none of these repositories is modified by this change:

| Change group | Repositories | Required proposal change |
|---|---|---|
| Shared platform | `dotnet-platform-libs` | Remove SDK 8 and `net8.0`; align packages, templates, manifests, code comments, docs, and gates to .NET 10 |
| SDK-inherited `net10.0` | `arivio`, `campusMesh`, `chinago`, `dharmatlas`, `hestiaLab`, `kairosa`, `lexora`, `linglitch`, `loreMix`, `namevia`, `orphevia`, `rostera`, `triplanify` | Remove inherited SDK 8 and update remaining package, runtime, CI, code, and documentation surfaces |
| SDK-inherited `net8.0` | `aegify`, `aesthra`, `bargora`, `catchen`, `crossalheart`, `crossify`, `hypora`, `mewo`, `ploutify`, `reelio`, `singleatee` | Change targets, Microsoft packages, code/tooling, runtime images, tests, migrations, and technical docs to .NET 10 |
| Local SDK 8 | `careBridge`, `fotofy`, `hermora`, `opendockify`, `openlearning`, `smotoox`, `somodanote`, `stylify`, `sutravia`, `testify`, `trippify` | Replace local SDK 8 pins immediately and migrate remaining net8/package/doc surfaces |
| Mixed/already SDK 10 | `citylens`, `cvunify`, `mortalect`, `pangoman`, `tantalyn` | Audit and propose changes for any remaining 8.x targets, packages, runtime references, or current documentation |

The nested `openlearning/git-credential-manager` repository receives its own
proposal because it has an independent SDK pin and solution. Non-C# projects
under `/home/paul/code` are excluded. Each follow-up proposal MUST be created
and implemented in its own repository, with no cross-repository source edits.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace MSBuild defaults | `/home/paul/code/global.json`, `/home/paul/code/Directory.Build.props` | Shared SDK selection and default target framework for C# projects | Current SDK/default target are 8.x; must change only after the platform package baseline is available | Workspace owner; applies to descendants but not non-MSBuild projects | **extend shared owner** |
| Platform shared library | `global.json`, `Directory.Packages.props`, `src/`, `tests/`, `templates/` | Reusable package contracts, adapters, test toolkit, samples, and template | Current packages and manifests describe `net8.0`; SDK 10 packaging behavior needs explicit evidence | `dotnet-platform-libs`; independently packaged NuGet boundary | **extend shared owner** |
| Product repositories | `/home/paul/code/*/**/*.csproj`, local `global.json`, `Directory.Packages.props`, Dockerfiles | Product-local package graphs, EF migrations, runtime and release checks | No single product owns another product's entities, migrations, providers, or deployment | Each repository owns its own migration and release boundary | **adapt through a generic adapter** |
| Non-C# workspace projects | Python, Rust, and other non-MSBuild trees under `/home/paul/code` | No .NET SDK or target framework contract | Changing workspace MSBuild files must not alter them | Their existing language-specific owners | **keep local** |

## Capabilities

### New Capabilities

- `dotnet10-platform-baseline`: A reproducible .NET 10 shared-library baseline
  covering SDK selection, target frameworks, package generation, generated
  artifacts, and verification evidence.

### Modified Capabilities

- None. Existing platform API and architecture requirements remain in force;
  this change changes the supported build/runtime baseline, not public package
  semantics.

## Impact

- Affects the repository `global.json`, all platform `.csproj` target
  frameworks, central package versions, package/template manifests, samples,
  tests, scripts, and baseline documentation.
- Public package consumers must rebuild against `net10.0`; this is a breaking
  target-framework/package-generation change.
- SDK 8, `net8.0`, Microsoft 8.x framework packages, and current technical
  documentation claiming they are the supported baseline MUST be removed from
  affected projects. No compatibility bridge is promised.
- Pure contracts remain free of ASP.NET Core and EF Core dependencies.
- Product entities, migrations, provider IDs, invoice rules, credentials,
  deployment configuration, and application-specific package choices remain
  outside this repository.
- No consumer repository is changed by this package; each consumer receives a
  separate migration change in its own repository after this package is
  verified.
