# .NET platform libraries module

The former `dotnet-platform-libs` repository is maintained as ordinary source
under [`modules/dotnet-platform-libs/`](../modules/dotnet-platform-libs/). Its
source revision and all source-only commits are retained in Forge Git history.
The module keeps its native solution, pinned .NET SDK, package identities,
tests, UI packages, local tooling, and independently runnable verification
scripts. It is not a Git submodule and does not require the archived sibling
checkout.

## Local verification

Run these commands from `modules/dotnet-platform-libs/`:

```sh
./scripts/quality-gate.sh
./scripts/check-public-api.sh
./scripts/conformance.sh
./scripts/audit-packages.sh
```

The root `forge gate` assesses the Forge Git repository. It does not treat a
nested module as a second Git project or independently run its nested Gate
manifest. The root .NET quality workflow owns module-scoped checks and runs
the native suite when module source, its root workflow, or the Forge Governance
project/UI path declarations change.

## CI and release

`.github/workflows/platform-dotnet-quality.yml` owns pull-request and `main`
quality runs. The release workflow is separate and can only be started
manually with an explicit SemVer input. It reruns quality, API, audit and
consumer checks before packing. Ordinary CI cannot publish packages or deploy
Pages.

The release workflow targets the Forge Pages feed at
`https://lileililiwen.github.io/forge/nuget/index.json`; package IDs remain
`Platform.*`. The canonical consumer config includes the old
`dotnet-platform-libs` feed as a fallback for versions already published
there. No old feed bytes were available from a source `gh-pages` branch during
the import, and this change does not assert the new feed is already deployed.
Before the first manual release, verify the repository's Pages configuration
and the legacy feed's availability.

## Ownership

Forge owns the monorepo registry and root Actions workflow names. The module
owns its .NET APIs, tests, package metadata, and UI package artifacts.
Independent package identities and release checks remain intact. The archived
source checkout is not a runtime or build dependency.
