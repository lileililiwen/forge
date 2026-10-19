## Context

`dotnet-platform-libs` contains framework-neutral contracts, optional ASP.NET
Core and EF Core adapters, test-only packages, samples, and an application
template. The repository currently pins SDK `8.0.424`, targets `net8.0`, and
centrally manages several Microsoft packages in the 8.x generation. The
workspace has SDK `10.0.400` installed and consumers are being moved to a
single .NET 10 baseline.

The change is repository-local. It does not edit product entities, migrations,
consumer repositories, provider credentials, or deployment environments.

## Goals / Non-Goals

**Goals:**

- Make every platform project, test, sample, tool, and template build for
  `net10.0` with SDK `10.0.400`.
- Align Microsoft ASP.NET Core, EF Core, and applicable Extensions packages to
  compatible 10.x versions while preserving package ownership and dependency
  direction.
- Keep generated manifests, consumer bootstrap conditions, template output,
  documentation, and verification commands consistent with the baseline.
- Produce explicit downstream migration evidence and a rollback point.

**Non-Goals:**

- Migrating any application repository under `/home/paul/code`.
- Changing public API semantics, package IDs, package ownership, or platform
  domain contracts.
- Upgrading unrelated third-party dependencies solely because their version
  string contains `8`.
- Adding ASP.NET Core or EF Core dependencies to framework-neutral packages.

## Decisions

### 1. Use an exact SDK pin

Set repository `global.json` to `10.0.400`, retain `allowPrerelease: false`,
and use `rollForward: latestPatch`. An exact feature-band pin gives all local
and offline workers the same compiler/MSBuild behavior while allowing only
patch-level servicing updates. Floating `10.x` or latest-major resolution is
rejected because it weakens reproducibility.

### 2. Make `net10.0` the only platform target

Change explicit project targets, inherited defaults, sample metadata, template
projects, fixtures, and the `Platform.Consumer.props` eligibility condition to
`net10.0`. Remove `net8.0` rather than multi-targeting: dual-targeting would
retain the 8.x compatibility surface and violate the immediate-break policy.

### 3. Align Microsoft package generations centrally

Update central Microsoft ASP.NET Core, EF Core, and applicable
`Microsoft.Extensions.*` package versions to the compatible 10.x line. Keep
third-party packages unchanged unless restore or compilation proves a specific
compatibility issue; any such change must name the package, reason, and test.
The platform's public package boundaries and framework-neutral dependency rules
remain authoritative.

### 4. Regenerate derived artifacts from source

Treat project files and central package metadata as source of truth. Regenerate
`eng/package-manifest.json` through the repository generator and update sample
and template assertions from source. Do not hand-edit EF migration product
version annotations or claim that a text replacement is an EF migration
upgrade; regenerate/validate them with EF tooling 10 where needed.

### 5. Verify offline in dependency order

Restore once using the configured local/cache-compatible source policy, then
run serial build, test, pack, architecture, package-manifest, public-API, and
strict OpenSpec checks. Use `-m:1` because parallel MSBuild/VSTest has caused
environment socket failures in this workspace. A missing local .NET 10 package
is `BLOCKED`, not a passing offline result.

### 6. Preserve a downstream contract, not consumer edits

Document the exact consumer prerequisites: SDK pin, target framework, package
generation, Docker/CI runtime images, EF tooling/migration validation, and
template smoke tests. Consumer repositories receive separate changes after the
shared packages are verified. Those changes are authored and implemented inside
their respective repositories; this package does not modify them.

Every downstream proposal MUST include a documentation-consistency task. A
repository is incomplete while current README files, runbooks, architecture
documents, CI guides, migration guides, or code comments still claim SDK 8 or
.NET 8 is the supported baseline. Historical release evidence may remain only
when clearly marked historical.

## Risks / Trade-offs

- **[Risk]** SDK 10 previously exposed a silent `dotnet pack --no-build`
  failure in `_GetFrameworkAssemblyReferences`. → **Mitigation:** require
  exact restore/build/test/pack verification under SDK 10, including a clean
  pack after the build.
- **[Risk]** Microsoft 10.x packages can change compile-time APIs or analyzer
  behavior. → **Mitigation:** update centrally, compile all projects, run all
  tests, and review public API/package dependency output.
- **[Risk]** Offline caches may not contain required .NET 10 packages. →
  **Mitigation:** inventory restore sources before implementation and record
  the exact missing package/source as a blocker.
- **[Risk]** Templates, fixtures, or generated manifests can remain on net8.0.
  → **Mitigation:** add repository-wide exact searches and generated-manifest
  checks to the final BFS phase.
- **[Risk]** Existing uncommitted OpenSpec/user files are overwritten. →
  **Mitigation:** stage only this change's paths and preserve all unrelated
  worktree entries.

## Migration Plan

1. Capture the current worktree and baseline package/API/manifest evidence.
2. Verify SDK `10.0.400` and local .NET 10 restore assets are available.
3. Update SDK, target frameworks, central Microsoft packages, bootstrap rules,
   templates, samples, scripts, and documentation.
4. Restore, build, test, pack, regenerate/check derived artifacts, and run
   strict validation.
5. Publish the verified downstream migration contract in the change evidence.
6. If any mandatory gate fails, leave the change active with the exact command,
   failure, affected project, and next action; do not archive.

Rollback is a source revert of this change to SDK `8.0.424`, `net8.0`, and the
previous Microsoft package versions. No database or deployed runtime migration
is performed by this repository change.

## Open Questions

- The compatible exact patch versions for Microsoft 10.x packages must be
  selected from the configured offline feed/cache during implementation and
  recorded in the final diff; `10.0.0` is not assumed to be available.
- EF migration projects must confirm whether any snapshot regeneration is
  required after the package update; the owning project evidence decides this.
