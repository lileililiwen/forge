# Platform Consumer Adoption

Repeatable, evidence-based adoption of the platform packages from a consumer
repository. The adoption path uses packed NuGet artifacts (not source
references) and exercises registration replacement, health, failure safety,
opt-in boundaries, and upgrade/rollback before any code is shipped.

## What this document covers

- Local/private feed setup for packed artifacts.
- Exact-version pinning and the rationale for rejecting floating ranges.
- **Shared MSBuild policy: the `Platform.Build` policy package.**
- A pilot-consumer fixture that exercises the adoption path.
- Upgrade and rollback smoke testing against packed artifacts.
- Vulnerability review and the production/test dependency rules.
- A per-repository adoption checklist.

## Shared MSBuild policy: the `Platform.Build` package

**This is the supported way for a consumer repository to obtain the platform's
shared MSBuild policy.** Obtain it by referencing the packed package at an exact
version:

```xml
<PackageReference Include="Platform.Build" Version="0.1.0" />
```

No import, no file copy, and no path to configure. The package ships
`buildTransitive/Platform.Build.props`, which NuGet imports automatically, so
the version a project was built against is recorded in the project's dependency
declaration rather than in an unreviewed file.

Do **not** obtain build policy from `/home/paul/code/Directory.Build.props`.
That file lives outside any Git repository, so no commit records it and no
review sees it; MSBuild finds it only by walking up the directory tree, which
means a clone on a different machine, or a repository moved one directory up,
silently changes the build. A package reference keeps the project buildable
with or without this repository present on disk.

### What the package asserts, and what it deliberately does not

| Property | Asserted value | Notes |
|---|---|---|
| `EnforceCodeStyleInBuild` | `true` | code style is enforced during build |
| `Deterministic` | `true` | reproducible output, so a packed package is comparable to its local build |
| `CentralPackageTransitivePinningEnabled` | `true` | a transitive dependency cannot slip past the pinned version set |
| `ContinuousIntegrationBuild` | `true` | only when `'$(CI)' == 'true'` |

Every one is set with `Condition="'$(X)' == ''"`, so **a consumer's own value
always wins**. Setting `EnforceCodeStyleInBuild` to `false` in your project is
enough to opt out of that single property.

The package asserts **no `TargetFramework` and no `RootNamespace`**. Those are
project identity: a shared package that asserted a TFM would retarget your
project behind your back, and generators that write a `.csproj` already own
both. The package also does not assert `PlatformAsSource`,
`PlatformPackageVersion` or `PlatformConsumerSourceRoot` — those depend on a
local filesystem probe of a sibling checkout, so they vary per machine and
belong to your own build, not to a feed-restored artifact.

The `NoWarn` suppression list is owned by the package and currently ships
**empty**: the baseline this package replaces carried no suppression list, so
there is no inherited set to justify. An entry may only be added together with
a one-line justification, and `BuildPolicyConformanceTests` rejects the packed
props if an unjustified entry ever appears.

### Single ownership: do not duplicate these five

`build/Platform.Consumer.props` already owns `Nullable`, `LangVersion`,
`AnalysisLevel`, `TreatWarningsAsErrors` and `ManagePackageVersionsCentrally`.
The policy package must not assert them, and neither should a consumer's own
file, because two `== ''`-guarded files claiming one property resolve it by
import order and the loser is silent. `BuildPolicyConformanceTests` fails the
build if the packed props ever starts claiming one of the five.

If you want that consumer bootstrap, import it by path as documented in
[`docs/workspace-consumer-bootstrap.md`](workspace-consumer-bootstrap.md). The
two routes are complementary and are documented in both directions:

| Route | Document | Obtains |
|---|---|---|
| Policy package (restore-driven, versioned) | this document | code style, determinism, central pinning, CI flag |
| Consumer bootstrap (import by path, opt-in) | [`workspace-consumer-bootstrap.md`](workspace-consumer-bootstrap.md) | the five consumer defaults plus the implicit `Platform.Core` reference |

### Verifying the policy actually applied

A build-only package restores and builds identically whether or not its props
are honoured, so a green build is **not** evidence that the policy applied.
`tests/Platform.ConsumerConformance/Tests/BuildPolicyConformanceTests.cs` reads
the *resolved* MSBuild property values from a fixture that restores
`Platform.Build` out of the feed, and additionally lists the packed archive to
prove `buildTransitive/Platform.Build.props` is really inside the `.nupkg`.
Run it with:

```bash
bash scripts/conformance.sh
```

Note that replacing a packed artifact in `.local-feed` is not enough on its
own: the fixtures must be re-restored, because MSBuild imports the *extracted*
copy under the conformance project's `.nuget-packages` directory. A missing
extracted props file is imported as a no-op, which is the silent-degradation
failure this route exists to remove.

## Local / private feed setup

The platform packs every project to a local feed at
`tests/Platform.ConsumerConformance/.local-feed` and the conformance
project restores from that feed only. The local feed is the
single source of truth for adoption evidence: every `dotnet pack` invocation
produces a `.nupkg` plus a `.snupkg` symbol package and the manifest script
captures the public metadata.

To run the suite against a private feed, override the source:

```bash
NUGET_SOURCE="https://pkgs.example.com/nuget/v3/index.json" \
  bash scripts/conformance.sh
```

The conformance fixture is also `nuget.config`-driven; the file is checked
in next to the project so consumer repositories can copy the layout
verbatim. The file ships with the local feed plus nuget.org as a fallback.

## Exact-version pinning

The conformance fixture pins every `Platform.*` reference to the same exact
version. Floating ranges, wildcards, and `[1.0,2.0)` ranges are rejected by
`AdoptionConformanceTests.Conformance_project_pins_every_Platform_package_to_an_exact_version`
and the equivalent architecture test. The rule is enforced at the consumer
end too: every consumer repository should adopt `ManagePackageVersionsCentrally`
with `<PackageVersion Include="Platform.AspNetCore" Version="0.1.0" />`
lines that pin every platform package to a single value. `Platform.Build` is a
`Platform.*` package and is bound by the same rule: its policy package
reference must name an exact version, never a range or a wildcard.

Rationale: the platform reserves the right to ship breaking changes in
minor versions, so a single commit may produce two incompatible
`0.1.x` versions. Floating ranges would silently roll a consumer over a
breaking change and defeat the rollback path.

## Manifest

`eng/package-manifest.json` is the machine-readable description of every
package: id, version, target frameworks, description, project references,
package references, and framework references. Generate it locally with
`scripts/generate-package-manifest.sh` and verify drift with
`scripts/generate-package-manifest.sh --check`. The check runs as a
`Platform.Architecture.Tests` test so the manifest is part of the
continuous-integration gate.

The conformance suite reads the manifest and asserts:

- Every `Platform.*` reference in the fixture maps to a manifest entry.
- Every packable package has a concrete version (no floating).
- Every `<X>.Testing` package has a matching `<X>.Contracts` (or `<X>`)
  partner, except `Platform.Testing`, which is the standalone test helper.

## Pilot consumer selection

The conformance fixture
(`tests/Platform.ConsumerConformance/Platform.ConsumerConformance.csproj`)
is the platform's own pilot consumer. It pulls every stable package via
`PackageReference` only, has no `ProjectReference` to any platform source,
runs the registration / replacement / health / opt-in suite against the
packed artifacts, and ships an upgrade/rollback smoke test. Every consumer
repository should be able to copy this project verbatim and pass the same
suite.

For real consumer repositories, the recommended pilot is a non-critical
service that does not own production traffic. The pilot should:

1. Add the platform packages with exact versions.
2. Run the consumer conformance suite against its own services.
3. Promote the package to other repositories only after the pilot
   passes the upgrade/rollback smoke test.

## Upgrade and rollback smoke test

`scripts/consumer-upgrade-rollback.sh` packs the candidate version, packs
the previous version, restores the fixture against the candidate feed,
runs the suite, then restores the fixture against the previous feed and
re-runs the suite. The script is driven entirely by the manifest, so the
"previous version" is always the latest packable version recorded at
archive time. The script supports `ALLOW_ENV_BLOCKER=1` to reclassify
feed failures as environment blockers (exit code 75) so that CI can
distinguish a real conformance regression from a network or feed outage.

When the candidate run fails, the script automatically rolls back to the
previous version and re-runs the suite. The script exits non-zero so
release publication is blocked until the failure is understood.

## Vulnerability review and the production/test dependency rule

`scripts/audit-packages.sh` runs `dotnet restore -p:NuGetAudit=true
-p:NuGetAuditMode=all -p:NuGetAuditLevel=high` against the solution and
reports `AUDIT_STATUS=VERIFIED` on success or `AUDIT_STATUS=UNVERIFIED`
on a timeout or feed failure. The `ALLOW_UNVERIFIED_AUDIT=true`
environment variable is required to release a package with an
unverified audit; releases are otherwise blocked.

The platform also enforces a strict production/test dependency rule:

- Production projects (`src/Platform.*.csproj` except `*.Testing.csproj`)
  must never `ProjectReference` a `*.Testing.csproj` project.
- The consumer conformance project must never `ProjectReference` any
  platform project, ever.
- The consumer conformance project must never publish its own
  `Platform.*` package (`<IsPackable>false</IsPackable>`).
- Production projects must never `ProjectReference` the consumer
  conformance project.

The `Platform.Architecture.Tests` project enforces all four rules.

## Per-repository adoption checklist

1. Decide on a non-critical pilot service.
2. Add the platform packages with exact versions and a single version
   set (no floating ranges, no per-package drift).
3. Copy the `nuget.config` layout from the platform so the local feed is
   the first source.
4. Copy the conformance fixture and run it against the pilot's services.
5. Run the upgrade smoke test against the candidate and previous
   versions.
6. Confirm `dotnet build` and `dotnet test` pass with the new versions.
7. Promote the package to other repositories one at a time, with the
   same smoke test in each.
8. Record the promotion in the consumer repository's `CHANGELOG.md`.

## Troubleshooting

- "Manifest drift detected" — run
  `scripts/generate-package-manifest.sh` and commit the regenerated
  file.
- "Pinned version conflict" — the consumer is using a different version
  for one of the `Platform.*` packages. Align the version in the
  consumer's `Directory.Packages.props` and rerun the suite.
- "Local feed not found" — the conformance script is supposed to
  populate `tests/Platform.ConsumerConformance/.local-feed`; check
  that `dotnet pack` ran without warnings.
- "Audit unverified" — the vulnerability audit timed out. Re-run the
  audit or set `ALLOW_UNVERIFIED_AUDIT=true` only with a documented
  risk acceptance.