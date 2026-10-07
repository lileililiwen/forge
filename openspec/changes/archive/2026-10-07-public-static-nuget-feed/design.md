# Design

## Ownership and endpoint

`dotnet-platform-libs` owns packaging and publication. The canonical initial
endpoint is
`https://lileililiwen.github.io/dotnet-platform-libs/nuget/index.json`.
Sleet 7.2.0 generates the static NuGet v3 tree under `nuget/`; GitHub Pages
serves that tree anonymously. Forge may generate conforming files later, but no
consumer needs Forge to restore or build.

## Consumer configuration

The canonical committed `NuGet.config` declares `platform` and `nuget.org`
without `<clear />`, allowing a user-level `platform-local` source to merge.
Package Source Mapping maps `Platform.*` to `platform` and `*` to nuget.org.
The user-level configurator adds `platform-local` and maps `Platform.*` to it.
NuGet may therefore consider the trusted local and public platform feeds, but a
changed local package must carry a unique prerelease version that does not exist
remotely. Released versions remain immutable.

The consumer owns one defaulted property:

```xml
<PlatformPackageVersion Condition="'$(PlatformPackageVersion)' == ''">0.1.0</PlatformPackageVersion>
```

Every central or inline `Platform.*` package version uses that property. A
developer selects a local prerelease without editing a tracked file by passing
`-p:PlatformPackageVersion=<unique-prerelease>`.

## Publication and persistence

The manual release workflow performs quality, API, audit, pack and conformance
checks before publication. It installs the repository-pinned Sleet tool,
restores the existing `gh-pages` feed tree when present, refuses an existing
package ID/version with different bytes, pushes the new `.nupkg` set, validates
the feed, commits the updated static tree to `gh-pages`, and deploys the same
tree through GitHub Pages. The workflow uses `contents: write`, `pages: write`
and `id-token: write`; it has no consumer token and no package-registry PAT.

The workflow records packages and validation output as Actions artifacts.
Pages enablement is an external repository setting and is reported as a remote
verification prerequisite rather than inferred from local validation.

## Legacy migration

`scripts/migrate-consumer-nuget.py` accepts a consumer root and defaults to a
read-only check. `--apply` writes only after the complete repository validates:

1. Locate `Platform.*` references in `Directory.Packages.props` and `*.csproj`.
2. Refuse floating versions, missing versions, or multiple platform versions.
3. Write the canonical `NuGet.config` atomically.
4. Add the defaulted `PlatformPackageVersion` property to the central package
   file when present, otherwise to a repository `Directory.Build.props`.
5. Replace literal platform versions with `$(PlatformPackageVersion)`.
6. Remove recognized `PlatformAsSource` project-reference branches and make
   their existing package-reference branches unconditional.
7. Remove the legacy source-root properties from repository build props.
8. Re-run its own check and return non-zero on remaining drift.

The command never edits a sibling repository unless that repository is the
explicit target supplied by the operator. It never adds a project reference or
an absolute path.

## Local source configuration

`scripts/configure-local-source.py` receives the feed directory explicitly,
calls `dotnet nuget add source` against the current user's NuGet config, then
adds the `platform-local` source mapping idempotently. It refuses nonexistent
or non-directory inputs. It never touches a consumer repository. Removing the
source is performed with the standard `dotnet nuget remove source` command;
the helper also supports `--remove` to remove its mapping.

## Failure boundaries

- A package version collision with different bytes stops publication.
- A missing existing Pages branch initializes an empty feed; any other fetch
  failure stops publication rather than erasing history.
- A legacy repository with inconsistent platform versions receives a report
  and no writes.
- A local source without a unique prerelease version is unsupported and is
  called out by the skill and documentation.
- A cold consumer restore is tested with an isolated global package folder and
  no user-level configuration.
