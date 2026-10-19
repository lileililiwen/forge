# Tasks: One consumer bootstrap for the platform

## 0. Caveats and confirmations

- [x] **Read `openspec/specs/dotnet10-platform-baseline/spec.md` first.** It
  already asserts that a generated consumer "selects the expected local platform
  reference and the opt-out behavior remains explicit and testable". This change
  makes that assertion real. If any part is already satisfied, narrow the change
  instead of duplicating it, and never contradict that spec.
- [x] **Public API impact must stay none.** If implementation appears to need a
  public API change, stop and re-scope — the repository config requires stating
  this.
- [x] **Scope the diagnostic to opted-in consumers.** The workspace default target
  is `net8.0`. The unsupported-target diagnostic MUST fire only for a project that
  imports the platform bootstrap, or every default-target project under the
  workspace root would fail. The bootstrap is gated by the new
  `PlatformConsumerBootstrap=true` opt-in, so the diagnostic only fires for
  consumers that explicitly ask for it.
- [x] **Do not edit the workspace or the consumers.** `PlatformAsSource` and
  `PlatformPackageVersion` live in the workspace `Directory.Build.props`; `mewo`
  (net8.0) and `therapist-commons`/`citylens` (hand-copied defaults) are
  consumers. Document the contract; name migration as follow-up.
- [x] **Extend, do not replace, `platform-consumer-adoption`.** Its
  packed-artifact conformance, pinned-version and upgrade/rollback requirements
  stay as they are. The new `scripts/conformance.sh` keeps the packed-artifact
  pack/restore/build/test steps and appends bootstrap fixture exercises.
- [x] **No committed credential.** Publish uses the `GITHUB_TOKEN` expansion
  already declared in the workspace `nuget.config`; never commit a token. The
  release workflow reads `GITHUB_TOKEN` from secrets and fails loudly when it
  is absent.
- [x] **Stage only this change directory.** `.project.json` is already modified;
  keep it out of the change's commits.

## 1. BFS — Baseline and impact coverage

- [x] Inventory `build/Platform.Consumer.props`, the workspace
  `Directory.Build.props` (`PlatformAsSource`, `PlatformPackageVersion`), the
  repository `Directory.Build.props`/`Directory.Packages.props`, the conformance
  fixture, and the release workflow.
- [x] Confirm the existing specs (`dotnet10-platform-baseline`,
  `platform-consumer-adoption`) so this change extends rather than replaces them,
  and confirm no public package API changes.
- [x] Add fixture consumers (source mode, package mode, `net8.0`, opt-out)
  before implementation. `tests/Platform.ConsumerConformance/Fixtures/Bootstrap/`
  ships nine fixture projects plus a shared `BootstrapFixtureSource.cs`.
- [x] Record the dependency direction and the smallest adoptable boundary.
  The bootstrap is opt-in (`PlatformConsumerBootstrap=true`); the
  `Platform.Consumer.props`/`Platform.Consumer.targets` pair is the single
  import a consumer needs.

## 2. DFS — Requirement-by-requirement implementation

- [x] Make the source/package switch real in the bootstrap, including the
  missing-checkout error. `PlatformAsSource=true` injects a `ProjectReference`
  and validates the checkout; `PlatformAsSource!=true` injects a
  `PackageReference` at `$(PlatformPackageVersion)`; an unset version or
  missing checkout surfaces a named diagnostic.
- [x] Propagate consumer defaults (nullability, language version, analyzers,
  warnings policy, central package management) with the opt-out honoured. The
  defaults are applied only when the consumer has not set the property, so
  overrides always win.
- [x] Add the unsupported-target diagnostic. A `net8.0` (or any non-`net10.0`)
  opted-in consumer fails the build with a message naming both frameworks.
- [x] Establish one version source and extend `release.yml` to push to the
  declared feed. The version is pinned in `Directory.Build.props`
  (`VersionPrefix`/`Version`) and referenced by every consumer. The release
  workflow now invokes `dotnet nuget push` for every packed `*.nupkg` against
  `PLATFORM_PUBLISH_SOURCE` (or the workspace's GitHub Packages feed) when
  `GITHUB_TOKEN` is present, and fails loudly otherwise.
- [x] Extend `scripts/conformance.sh` and the fixture to cover both modes.
  The script packs the platform, restores/builds/tests the conformance
  project from the local feed, and additionally builds the source-mode
  fixture, restores/builds the package-mode fixture, builds the opt-out
  fixture, and asserts the unsupported-target fixture surfaces a named
  diagnostic.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run restore/build/test/pack for the platform solution.
- [x] Verify an opted-out consumer is untouched and a source-mode consumer uses
  source, not the feed. `BootstrapConformanceTests` covers the opt-out,
  source-mode, and package-mode cases.
- [x] Verify the architecture tests (production packages never reference
  `Platform.Testing`) and the public-API baseline are unchanged. `Platform.*`
  public APIs are not modified; `eng/public-api-baseline.txt` is unchanged.
- [x] Update `docs/workspace-consumer-bootstrap.md` and
  `docs/platform-product-adoption.md`. The bootstrap doc documents the full
  contract; the adoption doc cross-links to it.

## 4. Verification

- [x] `dotnet restore && dotnet build Platform.sln -c Release && dotnet test
  Platform.sln -c Release && dotnet pack Platform.sln -c Release -o artifacts/packages`.
- [x] `./scripts/conformance.sh` (both modes, plus the `net8.0` and opt-out
  fixtures).
- [x] `./scripts/quality-gate.sh`, `./scripts/check-public-api.sh`,
  `git diff --check`, and `openspec validate --changes --strict --no-interactive`.
- [x] Record actual outputs; report any environment-blocked check with its exact
  next action rather than as passing.
