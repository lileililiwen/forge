# Design: Shared MSBuild policy as a packed `Platform.Build` package

## 1. Implementation boundary

Three parts, one change. Only the first is implemented inside this repository.

| Part | Owner | In this repository? | Edit surface |
|---|---|---|---|
| 1. The `Platform.Build` package | `dotnet-platform-libs` | yes | new project, `buildTransitive/Platform.Build.props`, solution, conformance fixture, docs |
| 2. Kit descriptor confirmed set | `forge` | no — recorded dependency | `forge/src/kit/registry.rs` evidence fixture |
| 3. Consumer migration + root-file retirement | 26 repositories + workspace governance | no — recorded plan | one `PackageReference` per repo, then the root file |

**Dependency direction.** The policy flows one way: `dotnet-platform-libs`
defines it, `forge` references it, consumers restore it. `forge` never defines
MSBuild policy, and a consumer never redefines it. The package has no dependency
on `forge` or on any consumer.

**Smallest adoptable boundary.** One package, one props file, one
`PackageReference`. A consumer opts in by adding the reference; there is no
import, no file copy, and no path to configure.

## 2. Language and runtime

.NET 10, matching the workspace baseline and the platform's existing target
(`forge/src/kit/registry.rs:459` pins `PLATFORM_TFM = "net10.0"`, and this
repository's projects declare `<TargetFramework>net10.0</TargetFramework>` per
project). `global.json` pins SDK `10.0.400` with `rollForward: latestFeature`.

`Platform.Build` is a **build-only** package: it carries MSBuild props and no
runtime assembly. No precedent for this pattern exists in this repository — a
search for `buildTransitive`, `IncludeBuildOutput` and `DevelopmentDependency`
under `src/` and `templates/` returns no source-level match. The pack shape
(`IncludeBuildOutput=false` plus a `None` item packed to `buildTransitive/`) is
therefore new here and is the first thing the fixture must prove.

## 3. Ownership and shared code — the one real design risk

`build/Platform.Consumer.props:93-99` **already** sets five of the properties
this package is meant to ship:

```xml
<Nullable Condition="'$(Nullable)' == ''">enable</Nullable>
<LangVersion Condition="'$(LangVersion)' == ''">latest</LangVersion>
<AnalysisLevel Condition="'$(AnalysisLevel)' == ''">latest-recommended</AnalysisLevel>
<TreatWarningsAsErrors Condition="'$(TreatWarningsAsErrors)' == ''">true</TreatWarningsAsErrors>
<ManagePackageVersionsCentrally Condition="'$(ManagePackageVersionsCentrally)' == ''">true</ManagePackageVersionsCentrally>
```

Two files now assert the same properties. Because both are `== ''` guarded,
first-writer-wins, and the winner is decided by **import order**:

- a `buildTransitive` props file is imported **before** the project body;
- the workspace `Directory.Build.targets` imports `Platform.Consumer.props`
  **after** the project body.

So for a repo under the workspace root, `Platform.Consumer.props` wins for those
five properties, and the package's copy is a no-op. The values are currently
identical, so nothing breaks today — but two files that both claim to own
`TreatWarningsAsErrors` is exactly the drift the change exists to end.

**Rule: single ownership per property.** Assign each property to exactly one
home and forbid the other from asserting it:

| Property | Owner | Rationale |
|---|---|---|
| `Nullable`, `LangVersion`, `AnalysisLevel`, `TreatWarningsAsErrors`, `ManagePackageVersionsCentrally` | `build/Platform.Consumer.props` (unchanged) | already shipped, already opt-in, already documented in `docs/workspace-consumer-bootstrap.md`; do not duplicate |
| `EnforceCodeStyleInBuild`, `Deterministic`, `NoWarn`, `ContinuousIntegrationBuild`, `CentralPackageTransitivePinningEnabled` | `Platform.Build`'s `buildTransitive` props | not asserted by `Platform.Consumer.props` today; these are the genuinely new contents |
| `TargetFramework`, `RootNamespace` | the consuming project | never asserted by the package — see §4 |

This split is the smallest change that removes the unowned baseline without
creating a second unowned one. It is recorded as a requirement
(`platform-build-policy`: *policy ownership shall be single*).

The import-order property itself is inherited from the change owner's
measurement. It is not treated as verified: an empirical fixture comparing the
resolved value under both import paths is a task, and the rule above is written
to be safe **either way**, because it forbids overlap rather than relying on who
wins.

## 4. What the props assert, and what they must not

**Assert** (the new contents, owned by the package):

- `TreatWarningsAsErrors` / `EnforceCodeStyleInBuild` / `Deterministic` — the
  warnings, style-enforcement and reproducibility policy.
- `NoWarn` — the justified suppression list. Every entry MUST carry a one-line
  justification, which is a convention already established in the shadowing
  repos (e.g. `smotoox/Directory.Build.props`: "Each entry MUST carry a one-line
  justification so future reviewers can re-evaluate") and in this repository's
  quality gates. An unjustified entry is a review failure, not a nit.
- `CentralPackageTransitivePinningEnabled` — keeps central pinning transitive.
- `ContinuousIntegrationBuild` under a CI condition — reproducible CI output.

Each property is set with `Condition="'$(X)' == ''"` so a consumer's own value
always wins, matching the existing `Platform.Consumer.props` convention.

**Must not assert:**

- `TargetFramework` — a shared package asserting a TFM on its consumers is
  heavy-handed, and `forge` already writes the TFM into the generated `.csproj`
  from the kit descriptor (`mod.rs:858-860`, `mod.rs:871`). A project can still
  override a TFM the package asserts because `buildTransitive` props import
  before the project body, but the package should not assert one.
- `RootNamespace` — same reason: it is per-project identity, already written by
  the generator.
- `PlatformAsSource` / `PlatformPackageVersion` / `PlatformConsumerSourceRoot` —
  these live in `/home/paul/code/Directory.Build.props` and depend on the local
  filesystem (`Exists(...)` on a sibling checkout). A property that varies per
  machine and per checkout does not belong in a versioned, feed-restored
  artifact. They remain the consumer's own concern.

## 5. Behavioral model

For a consuming project that adds `<PackageReference Include="Platform.Build" />`
at an exact version:

1. Restore resolves the package from the feed at the pinned version.
2. NuGet imports `buildTransitive/Platform.Build.props` before the project body.
3. Each owned property is set only if the consumer has not set it.
4. The consumer's project body runs and may override any of them.
5. If the workspace root `Directory.Build.targets` is also in scope,
   `Platform.Consumer.props` runs afterwards and owns the five properties
   assigned to it in §3.

A consumer that does not reference the package is unaffected — the props file
exists only inside the package.

## 6. Contract and compatibility

- **Exact-version pinning**, with floating ranges and wildcards rejected, per
  `docs/platform-consumer-adoption.md:37-50`. The package is a `Platform.*`
  package and is subject to the same rule.
- **The existing conformance machinery is extended, not replaced.**
  `scripts/conformance.sh:86` packs `Platform.sln` into
  `tests/Platform.ConsumerConformance/.local-feed` (currently 76 `Platform.*.0.1.0`
  packages, all at `0.1.0`). Packing the solution is what puts a new project into
  that feed, so the new project must be added to `Platform.sln`; no new pack
  script is introduced.
- **Restore alone does not prove the props applied.** A build-only package
  restores successfully whether or not its props are honoured, so the fixture
  must assert a *resolved property value*, not just a successful build.
- **Docs.** `docs/platform-consumer-adoption.md` gains a section naming the
  package the supported route to shared MSBuild policy, cross-linking
  `docs/workspace-consumer-bootstrap.md` for the import-by-path route so the two
  are discoverable from either direction.

## 7. Failure and boundary policy

- A consumer that references the package but cannot restore it must fail at
  restore, exactly as for any other pinned `Platform.*` package. No fallback to
  the unversioned baseline, and no silent degradation to "no policy".
- A prop file that fails to import must fail the build. A silently skipped props
  file is indistinguishable from no policy, which is the failure mode this change
  removes.
- Rollback is by version: revert the `PackageReference`, or pin the previous
  package version. `scripts/consumer-upgrade-rollback.sh` already rehearses
  candidate-vs-previous restore for the existing packages and is the oracle for
  this one.
- **Root-file deletion precondition.** `/home/paul/code/Directory.Build.props`
  is deleted only after all 26 inheriting repositories are verified migrated and
  a restore from a clone without the file succeeds. The file is never deleted
  while a consumer remains — a deleted baseline with a live consumer fails
  *worse* than a duplicated one, because the failure is a missing TFM rather than
  a divergent policy.

## 8. Verification oracle

| Claim | Oracle |
|---|---|
| package packs to the local feed | `dotnet pack` via `scripts/conformance.sh` produces `Platform.Build.<version>.nupkg` in `.local-feed` |
| props are applied | a fixture consumer asserts the resolved value of each owned property, e.g. via `$(TreatWarningsAsErrors)` printed by an MSBuild target — not merely a passing build |
| consumer override wins | a second fixture sets one owned property and asserts its value survives |
| package asserts no TFM | a fixture on a non-default TFM builds unchanged |
| exact version | the existing `platform-consumer-adoption` pinned-version test, extended to the new package |
| no policy regression in this repo | `dotnet build`/`test` on `Platform.sln` plus `scripts/quality-gate.sh` and `scripts/check-public-api.sh` |
| no drift with `Platform.Consumer.props` | a fixture under the workspace root asserts the resolved value matches the §3 ownership table |

## 9. Decision ledger

| Decision | Alternative rejected | Reason |
|---|---|---|
| Ship policy as a `buildTransitive` package | forge template | 25+ frozen copies that drift, none recording the version built against |
| Ship policy as a `buildTransitive` package | copy the root file per repo | that is the status quo: unowned by any commit, found only by an implicit walk |
| Package owns policy, not TFM / `RootNamespace` | assert the TFM too | heavy-handed on consumers; the generator already writes both; overridable anyway because the import is early |
| Single ownership: `Platform.Consumer.props` keeps its five properties | duplicate them in the package | both are `== ''` guarded; overlap makes the winner depend on import order and invites silent drift |
| 23 self-sufficient repos left alone | migrate all 49 | they already shadow; migrating them means reviewing 23 deliberate local baselines for no benefit, and would convert silent-but-harmless into 23 reviewed diffs |
| `forge` and the 26 repos recorded, not edited | edit them here | different repositories; and the preceding archived change set the same boundary ("no edits to any consumer repository") |
| Root-file deletion gated on verified migration | delete now | a live consumer of a deleted baseline fails worse than a duplicated one |

### Open, and deliberately not claimed

- **Import order of `buildTransitive` props vs. the project body** is inherited
  from the change owner's measurement, not verified here. §3 is written to be
  safe either way, and an empirical fixture is task 2.6.
- **The `NoWarn` list's exact contents** are not fixed by this change. The root
  file has no such list, so the package's list is a new artifact whose entries
  each need their own justification. Task 2.5 is unchecked for this reason.
- **Whether `Platform.Build` reaches the confirmed set at all** depends on the
  frozen evidence fixture in `forge` and on real repos having adopted it. It is
  provisional until then by construction (`registry.rs:349-360`).
- **Two changes are already active** (`ui-shared-layer-reachability` at 60/64
  tasks, `shared-test-web-factory` complete but unarchived), so `AGENTS.md`'s
  "one active OpenSpec change at a time" rule is already strained. This change
  adds a third. Recorded as a sequencing note in `tasks.md` §0 rather than
  resolved here.
