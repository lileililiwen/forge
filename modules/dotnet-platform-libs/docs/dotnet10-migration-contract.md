# .NET 10 downstream migration contract

This contract belongs to `platform-dotnet10-baseline`. It orders consumer
migrations and defines acceptance evidence. It edits no consumer repository.

## Order

1. `platform-dotnet10-baseline` (this repository) — verified first.
   Produces usable .NET 10 NuGet packages before any consumer adopts them.
2. `dotnet10-consumer-baseline` (workspace MSBuild boundary at
   `/home/paul/code`) — root `global.json`, `Directory.Build.props`, and
   consumer-local overrides move to the .NET 10 defaults. Authored and
   implemented in the workspace scope, not here.
3. `<consumer>-dotnet10-migration` (one change per application repository) —
   each repository migrates its code, packages, runtime images, and migrations
   inside its own boundary. Authored and implemented per repository, not here.

## Per-migration requirements

Each consumer migration package MUST cover:

- SDK pin: repository `global.json` pins SDK `10.0.400` with
  `rollForward: latestPatch` and `allowPrerelease: false`. No SDK 8 pin remains.
- Target framework: all C# projects target `net10.0`. No `net8.0` target remains
  in source-controlled projects, templates, samples, or fixtures.
- Microsoft packages: ASP.NET Core, EF Core, and applicable
  `Microsoft.Extensions.*` references resolve to the compatible 10.x generation.
  Third-party versions change only with a documented restore or compile failure.
- Docker/CI: runtime, SDK, and tooling images reference .NET 10. No .NET 8
  runtime or SDK image remains in current instructions.
- EF tooling: `dotnet-ef` 10.x validates migrations where applicable. Textual
  `ProductVersion` replacement is not migration evidence.
- Offline restore: restore succeeds from the configured package source or cache.
  A missing .NET 10 asset is `BLOCKED`, not a pass.
- Repository-native verification: restore, serial build (`-m:1`), test, pack,
  and the repository's own quality gates pass under SDK `10.0.400`.
- Documentation: current READMEs, runbooks, and code comments identify SDK 10 /
  .NET 10. Historical release evidence stays labeled historical.

## Consumer inventory (follow-up proposals only)

None of these repositories is modified by this change. Each group needs its own
proposal in its own repository. Non-C# projects are excluded.

| Group | Repositories |
|---|---|
| SDK-inherited `net10.0` | `arivio`, `campusMesh`, `chinago`, `dharmatlas`, `hestiaLab`, `kairosa`, `lexora`, `linglitch`, `loreMix`, `namevia`, `orphevia`, `rostera`, `triplanify` |
| SDK-inherited `net8.0` | `aegify`, `aesthra`, `bargora`, `catchen`, `crossalheart`, `crossify`, `hypora`, `mewo`, `ploutify`, `reelio`, `singleatee` |
| Local SDK 8 | `careBridge`, `fotofy`, `hermora`, `opendockify`, `openlearning`, `smotoox`, `somodanote`, `stylify`, `sutravia`, `testify`, `trippify` |
| Mixed / already SDK 10 | `citylens`, `cvunify`, `mortalect`, `pangoman`, `tantalyn` |

The nested `openlearning/git-credential-manager` repository receives its own
proposal because it has an independent SDK pin and solution.

## Acceptance evidence per package

| Package | Independent oracle |
|---|---|
| `platform-dotnet10-baseline` | Full platform restore, serial build, test, pack, architecture, manifest, and strict OpenSpec checks under SDK `10.0.400`; representative package dependency groups target `net10.0` with 10.x Microsoft dependencies and no 8.x runtime package dependencies |
| `dotnet10-consumer-baseline` | Inventory has no targeted 8.x SDK/TFM declarations; representative projects restore and build |
| `<consumer>-dotnet10-migration` | Repository-native restore, build, test, runtime or migration checks; application-owned EF migrations, provider configuration, and runtime images validated in place |

## Rollback

This repository rolls back by reverting to SDK `8.0.424`, `net8.0`, and the
previous Microsoft package versions. No database or deployed runtime migration
is performed by this repository change. Consumer rollbacks are owned by their
respective repositories.
