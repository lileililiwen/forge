# Platform template pack

Installable `dotnet new` template for a minimal platform-backed ASP.NET
Core application. The template generates detached, application-owned
source with no runtime dependency on the template pack.

## Install and generate

```bash
dotnet new install Platform.Application.Template --nuget-source <feed>
dotnet new platform-app -n Acme
```

The template pack version tracks the platform package version (`0.1.0`);
generated projects pin exact versions (`Platform.* 0.1.0`,
`Microsoft.EntityFrameworkCore.Sqlite 10.0.0`,
`Microsoft.AspNetCore.Mvc.Testing 10.0.0`,
`Microsoft.NET.Test.Sdk 17.11.1`, `xunit 2.9.2`,
`xunit.runner.visualstudio 2.8.2`).

## Variants

| Symbol | Default | Effect |
| --- | --- | --- |
| `IncludeTests` | `true` | Generates `tests/<Name>.Tests` with in-process HTTP smoke tests (`/` and `/live`). `false` omits the `tests/` tree. |
| `EnableIdentity` | `false` | Enables the platform identity lifecycle capability in `AddPlatformApplication` (missing-provider fallbacks; application-owned stores stay application-owned). |
| `EnablePersistence` | `false` | Adds `Platform.Persistence.EfCore` plus an application-owned SQLite provider reference, an application-owned `Data/AppDbContext`, SQLite connection string defaults, and the platform audit/soft-delete interceptor registration. Providers, migrations, and seed data stay application-owned. |

All eight combinations are valid and covered by
`tests/Platform.Template.Tests` (pack, install, generate, build, test).
There are no React, Aspire, Docker, Terraform, product module, or
platform source copies in any variant.

```bash
dotnet new platform-app -n Acme --IncludeTests false --EnableIdentity true --EnablePersistence true
```

## Generated ownership

Generated source is application-owned from creation:

- No project reference to the template pack and no
  `.template.config` metadata leak into the output.
- The generated `README.md` records the detachable versions and the
  uninstall guarantee.
- Uninstalling the template pack (`dotnet new uninstall
  Platform.Application.Template`) never affects generated source:
  restore, build, and test keep working from the application's own
  source plus the platform package feed.

## Package pinning

Generated projects are detached from `Directory.Packages.props`, so
every reference carries an exact `Version` attribute. When the platform
`VersionPrefix` moves, update the pinned `Platform.*` versions in
`templates/platform-application-starter/**/*.csproj` together with the
template pack version, and re-run the template smoke tests. The
architecture suite guards exact pinning, content-tree containment, and
the minimal-output markers.

## Rollback

- Template consumers: `dotnet new uninstall
  Platform.Application.Template`, or delete the generated source. No
  application runtime dependency is ever added by installing the pack.
- Template authors: the tracked content tree is
  `templates/platform-application-starter/`; the pack-only project is
  `templates/Platform.Application.Template/` (`PackageType=Template`,
  content-only, explicit `bin`/`obj` excludes). Re-pack and reinstall to
  roll forward.
