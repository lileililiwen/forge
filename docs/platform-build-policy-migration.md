# Shared MSBuild policy: migration and root-file retirement

The migration plan for the `Platform.Build` policy package, and the
precondition under which the unversioned workspace baseline may be deleted.

**The consumer migration in this document has been performed** (2026-10-03,
part 3 of the change). All 25 repositories carry a pinned `Platform.Build`
reference, each in its own commit on `main`. **The root file has not been
deleted**: three of the five deletion clauses are still unmet, recorded below.
`Platform.Build` ships the policy; the rollback manifest below is what undoes
the consumer half.

The inventory below was measured on 2026-10-03 by walking `/home/paul/code`
for Git repositories containing at least one `*.csproj` and comparing each
one against `/home/paul/code/Directory.Build.props`. The migration re-measured
the same way, resolving the baseline per project by walking up from each
project directory the way MSBuild does, and confirmed the 25/23 split.

## What is being retired

`/home/paul/code/Directory.Build.props` (19 lines) and its companion
`/home/paul/code/Directory.Build.targets` (11 lines) live outside every Git
repository: `/home/paul/code` has no `.git`, so no commit records either file,
no review sees a change to it, and no history explains it. MSBuild finds them by
walking up the directory tree, so every .NET repository beneath that root
inherits them whether or not it knows they exist. The replacement route, and why
it is the supported one, is documented in
[`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md).

## Measured inventory

| Group | Count | Meaning |
|---|---|---|
| Inherits the workspace baseline | **25** | ships no `Directory.Build.props`; the root file reaches its build today |
| Shadows it | **23** | ships its own `Directory.Build.props`; the root file never reaches its build |
| Total .NET Git repositories | **48** | |

Every one of the 25 inheriting repositories has **no `Directory.Packages.props`
at any depth** (re-measured during the migration), so central package management
is not available to any of them today, and a bare
`<PackageReference Include="Platform.Build" />` would not restore. The
version-declaration style is a per-repository decision, and the one that
satisfies [`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md)'s
exact-version rule without introducing a new central file is an inline `Version`
attribute on the `PackageReference` itself:

```xml
<PackageReference Include="Platform.Build" Version="0.1.0" />
```

Introducing `Directory.Packages.props` in a repository that has never had one
is a larger decision than this migration: it switches on central package
management for every reference in that repository, not just this one. Where a
repository already wants CPM, `<PackageVersion Include="Platform.Build"
Version="0.1.0" />` is equivalent and preferred. Both forms are exact; neither
uses a range or a wildcard.

**This is the form all 25 repositories use**, so the decision was uniform and
is recorded once: the majority of them already pin third-party references
inline, the inline attribute matches that existing shape, and no repository
gains a `ManagePackageVersionsCentrally` switch it did not ask for. It also
keeps each repository's whole change to a single line per project.

### Correction to the change's own inventory

`openspec/changes/platform-build-policy-package/proposal.md` records a
correction stating the count is 49 rather than 48, because "`forge` is itself a
.NET repository that also inherits the root file and was in neither bucket".
That correction does not hold. `forge` is a Rust project: it has a
`Cargo.toml`, no `global.json`, no solution, and no `*.csproj` outside
`target/portability/origin/net-app/`, which is generated build output rather
than a source project. MSBuild's directory walk would reach it if it had a .NET
project; it does not, so it is not a consumer of the baseline.

What remains true from that proposal bullet is the structural point: `forge`'s
`aspnet-web` generator emits no `Directory.Build.props`, so every *new scaffold*
inherits the unversioned baseline. That is a property of the generator's output,
not of `forge` itself, and it is the reason part 2 of the change matters.

The 23 shadowing repositories are **out of scope** and are not listed here. They
are already self-sufficient, and several document why they shadow
(`therapist-commons`, `citylens`, `devloom`, `kairion`, `mewo` — the last three
document the same specific escape, refusing the ambient `PlatformAsSource`
project-reference injection). Migrating them would mean reviewing 23
independent local baselines, each with deliberate deviations, for no benefit.
The accepted cost is that the shared policy is **not** uniform across the
workspace once this lands. That is stated rather than hidden.

## Migration manifest

One row per inheriting repository. Every row is the same mechanical change, so
the manifest is a checklist rather than a design; it is recorded here so the
migration is reviewable as a unit and so a partial migration is visible.

The version-declaration style was measured per repository before the migration
and turned out to be uniform: **all 25 use the inline `Version` attribute**,
because none of them has a `Directory.Packages.props` at any depth. The
"Version style today" column therefore records the style the migration
actually used, and the `.NET projects` column is the number of `.csproj` files
that received the reference. The counts in this table were re-measured during
the migration and match the original measurement exactly.

| # | Repository | .NET projects | Version style today | Migrated |
|---|---|---|---|---|
| 1 | aegify | 2 | none — inline `Version` used | yes — `3f92d20` |
| 2 | aesthra | 2 | none — inline `Version` used | yes — `e965b24` |
| 3 | arivio | 6 | none — inline `Version` used | yes — `f3075e1` |
| 4 | bargora | 2 | none — inline `Version` used | yes — `e050296` |
| 5 | campusMesh | 20 | none — inline `Version` used | yes — `8248ebd` |
| 6 | chinago | 2 | none — inline `Version` used | yes — `e0c79fa` |
| 7 | crossify | 2 | none — inline `Version` used | yes — `64322e4` |
| 8 | cvunify | 2 | none — inline `Version` used | yes — `7c32934` |
| 9 | fotofy | 16 | none — inline `Version` used | yes — `79ec60a` |
| 10 | hestiaLab | 2 | none — inline `Version` used | yes — `8d93744` |
| 11 | hypora | 2 | none — inline `Version` used | yes — `9e36d30` |
| 12 | kairosa | 2 | none — inline `Version` used | yes — `2c5540d` |
| 13 | lexora | 6 | none — inline `Version` used | yes — `7945945` |
| 14 | linglitch | 2 | none — inline `Version` used | yes — `7df21bf` |
| 15 | mortalect | 5 | none — inline `Version` used | yes — `d8f117d` |
| 16 | namevia | 5 | none — inline `Version` used | yes — `9663170` |
| 17 | orphevia | 5 | none — inline `Version` used | yes — `eabef4b` |
| 18 | pangoman | 2 | none — inline `Version` used | yes — `1fc912a` |
| 19 | ploutify | 4 | none — inline `Version` used | yes — `94189b2` |
| 20 | reelio | 2 | none — inline `Version` used | yes — `8f0979c` |
| 21 | retakeKit | 4 | none — inline `Version` used | yes — `5705828` |
| 22 | rostera | 6 | none — inline `Version` used | yes — `05e8adf` |
| 23 | singleatee | 8 | none — inline `Version` used | yes — `036afad` |
| 24 | sutravia | 3 | none — inline `Version` used | yes — `bc38d13` |
| 25 | triplanify | 2 | none — inline `Version` used | yes — `b693bff` |

Two repositories in this table already consume packed platform artifacts
(`triplanify` and `therapist-commons`; only `triplanify` is on this list), which
is why the feed transport is proven rather than assumed. `triplanify` is also
the only repository on this list that had a platform reference to preserve; see
"What the root file was actually providing" below.

Total: **25 repositories, 25 commits, 114 `.csproj` files.**

## Rollback manifest

### Per-repository commit pairs

Every repository took exactly one commit, on `main`, touching only `.csproj`
files. Reverting the whole migration is one command per repository:
`git revert <after>`, or `git reset --hard <before>` when the local branch
may be discarded. The `before` column is the commit to return to.

| # | Repository | before (revert to) | after (migration commit) |
|---|---|---|---|
| 1 | aegify | `86dde45` | `3f92d20` |
| 2 | aesthra | `9039e9a` | `e965b24` |
| 3 | arivio | `daa23eb` | `f3075e1` |
| 4 | bargora | `e636deb` | `e050296` |
| 5 | campusMesh | `48f0185` | `8248ebd` |
| 6 | chinago | `4557c6d` | `e0c79fa` |
| 7 | crossify | `49877eb` | `64322e4` |
| 8 | cvunify | `a54a391` | `7c32934` |
| 9 | fotofy | `4803722` | `79ec60a` |
| 10 | hestiaLab | `9e1ee17` | `8d93744` |
| 11 | hypora | `7b0f796` | `9e36d30` |
| 12 | kairosa | `5c8b610` | `2c5540d` |
| 13 | lexora | `1d732bd` | `7945945` |
| 14 | linglitch | `deece50` | `7df21bf` |
| 15 | mortalect | `ec25ac9` | `d8f117d` |
| 16 | namevia | `0395f03` | `9663170` |
| 17 | orphevia | `1e48fa0` | `eabef4b` |
| 18 | pangoman | `6430da9` | `1fc912a` |
| 19 | ploutify | `764eb0e` | `94189b2` |
| 20 | reelio | `4c0128d` | `8f0979c` |
| 21 | retakeKit | `79cb39e` | `5705828` |
| 22 | rostera | `a13d5e9` | `05e8adf` |
| 23 | singleatee | `f3ca585` | `036afad` |
| 24 | sutravia | `8a0fe64` | `bc38d13` |
| 25 | triplanify | `455858d` | `b693bff` |

All 25 commits are on `main`; no branch was created. None of them contains a
non-`.csproj` path, and each was staged by explicit path, so the uncommitted
work that already existed in five of these working trees
(`cvunify`, `hestiaLab`, `hypora`, `lexora`, `orphevia`) is still uncommitted
and is not part of any migration commit.

### Cost cases

Rollback is per repository and per version, and is deliberately cheap:

| Situation | Action | Cost |
|---|---|---|
| A migrated repository misbehaves | delete the `PackageReference` | the repository falls back to whatever baseline it had before, i.e. the root file if it is still there |
| The pinned `Platform.Build` version is wrong | change the version to the previous one | re-restore; `scripts/consumer-upgrade-rollback.sh` rehearses exactly this candidate-then-previous cycle |
| A rollback is needed after the root file is deleted | pin the previous `Platform.Build` version | the previous `.nupkg` must still be in the feed; a feed that drops old versions removes the rollback path |
| The policy itself is wrong | ship a new `Platform.Build` version and bump every reference | 25 edits, which is the real cost of this design and the reason the version set is deliberately narrow |

The last row is the risk this design accepts in exchange for a single owner per
property. A policy change is one edit here plus 25 version bumps, instead of
26 independent copies that drift. The alternative — per-repository
`Directory.Build.props` files — has no bump step and no drift protection at all.

## Precondition for deleting the root file

`/home/paul/code/Directory.Build.props` and
`/home/paul/code/Directory.Build.targets` may be deleted **only** when **all**
of the following hold. Any one of them failing means the file stays.

1. **All 25 rows in the manifest above read `yes`.** A single unmigrated
   repository is disqualifying, not a rounding error.
2. **Each migrated repository builds with the file absent.** Verified by a
   restore and build of a clone that does not contain it, not by reasoning about
   what the file does.
3. **Each migrated repository's build output is unchanged by the migration.**
   The policy the package asserts and the policy the root file provided are
   deliberately different sets of properties, so a repository may legitimately
   gain `EnforceCodeStyleInBuild` or `ContinuousIntegrationBuild` on CI. A
   repository whose build breaks on that gain has a local deviation that the
   migration must surface, not paper over.
4. **No repository in the workspace depends on the root file for
   `PlatformAsSource` / `PlatformPackageVersion` / `PlatformConsumerSourceRoot`.**
   Those three properties are the transport switch and the version pin. They
   stay behind in the repositories that need them: they depend on an `Exists()`
   probe of a sibling checkout and vary per machine, which is exactly why they
   are excluded from the package.
5. **`forge`'s generator emits the reference for new scaffolds.** Until it does,
   a scaffold created after the deletion inherits nothing, and nothing verifies
   that. This is part 2 of the change.

**The file must not be deleted while any consumer remains.** A deleted baseline
with a live consumer fails *worse* than a duplicated one: a duplicated baseline
produces a divergent policy that review can catch, while a missing one produces
a project with no target framework and no named diagnostic.

### Clause status after the migration

| # | Clause | Status | Evidence |
|---|---|---|---|
| 1 | All 25 rows read `yes` | **satisfied** | 25 of 25 migrated, 25 of 25 committed, 114 `.csproj` files changed in total |
| 2 | Each repository builds with the file absent | **not satisfied** | Not attempted. The root file is still present, so no build in the migration ran without it. See the note below. |
| 3 | Build output unchanged by the migration | **partially satisfied, one exception** | 24 of 25 build identically before and after. `retakeKit` was already red at its parent commit. `EnforceCodeStyleInBuild` moved `false` → `true` everywhere, which is the anticipated gain and broke nothing. |
| 4 | No repository depends on the root file for the three transport properties | **satisfied** | Measured, not assumed: see "What the root file was actually providing" below. |
| 5 | `forge`'s generator emits the reference | **not satisfied** | Part 2 of the change, not started. |

Clauses 2 and 5 are unmet, so **the root file stays**.

Clause 2 is unmet in a specific way worth recording, because it is cheap to
mistake for a pass. The migration verified that every project builds *with* the
root file present, at both the before and after commits, which is the opposite
test. Deleting the file and re-running the suite is a separate, destructive
experiment: it cannot be done while the file is still the only thing providing
the baseline, and doing it in place would take the 25 working trees with it.
The measurement that makes clause 2 likely to pass is recorded here instead:
no project in any of the 25 repositories relies on the root file for anything.

## What the root file was actually providing

This is the finding that shaped the migration, and it corrects the framing in
the section above. The root `Directory.Build.targets` imports
`build/Platform.Consumer.props`, but that file gates every one of its effects
on `PlatformConsumerBootstrap == 'true'`
(`build/Platform.Consumer.props:36,45,53,66,78,93,104,116`). The root
`Directory.Build.targets` does not set that property; it only decides whether
to import the file. Resolved values were measured across all 25 repositories
before the change:

| Property | Value in all 25, before the change |
|---|---|
| `PlatformConsumerBootstrap` | `""` (empty, in every project) |
| `PlatformConsumerOptOut` | `""` (empty, in every project) |

So the root targets file resolved to a no-op in all of them: it delivered
**no** implicit `Platform.Core` reference and **none** of the five consumer
defaults. The only repository with a platform reference was `triplanify`, and
it has its own explicit `PackageReference` lines for `Platform.Core` and
`Platform.AspNetCore`, which this migration did not touch. The other 24 have
no platform reference before or after, so none of them lost one.

The root `Directory.Build.props` did provide two things that mattered:

- `TargetFramework` defaulted to `net10.0` when a project did not set it. All
  114 projects set their own, so the default was dead code.
- `PlatformAsSource`, `PlatformPackageVersion` and
  `PlatformConsumerSourceRoot` were defined, and `PlatformAsSource` did
  resolve to `true` on this machine (the sibling checkout exists). Nothing
  consumed them, because the only consumer is the inactive bootstrap. That is
  what satisfies clause 4: no repository depends on the root file for any of
  the three.

## What the migration changed in each build

One line per project, `<PackageReference Include="Platform.Build"
Version="0.1.0" />`, appended to the existing `ItemGroup` that already holds
`PackageReference` items, or as a new `ItemGroup` where the project had none.
BOM and line endings were preserved per file, so 19 CRLF files kept CRLF.

Resolved properties were measured per project, before and after, with
`dotnet msbuild -getProperty:`. On all 114 projects `EnforceCodeStyleInBuild`
and `CentralPackageTransitivePinningEnabled` moved from the SDK default to
`true`, and `ContinuousIntegrationBuild` stayed empty locally and became `true`
under `-p:CI=true`, exactly as the packed props specifies. `Deterministic` was
already `true`, so it did not move.

`EnforceCodeStyleInBuild` becoming `true` is the one behavioural change, and it
is the gain clause 3 anticipates. It produced no new build errors in any of the
25 repositories, including the ones that set `TreatWarningsAsErrors` in their
own project body (`aegify` is one).

## One pre-existing breakage, not caused by this migration

`retakeKit` does not build, and did not build before the migration.
`apps/api` and `apps/api.tests` fail with 38 errors each at its parent commit
`79cb39e` and fail identically after `5705828`: `Services/SessionService.cs`
references `IClock`, `Error` and `Result<>`, and none of the three is declared
anywhere in the tree. Its other two projects, `packages/grading-sdk` and
`packages/grading-sdk.tests`, build clean with the reference.

The migration commit is still recorded, so the reference is present and
reviewable, and its message says plainly that it does not claim a green build.
It is reported here rather than hidden, because it is the one place where
"the build passes" is not a claim this document can make.

## Feed reachability

`Platform.Build 0.1.0` is currently resolvable on this machine from the global
packages folder (`~/.nuget/packages/platform.build/0.1.0`), and the cached
`.nupkg` is byte-identical to the one in
`tests/Platform.ConsumerConformance/.local-feed` (both
`sha256 6c258c003285aa04dd29db3316c60e1752704157045a0a2bccd87d664b2cf4d4`).
That is why the 25 repositories restore with no `nuget.config` change and no
committed feed path: a machine-local absolute path is exactly what the design
refuses to bake into a consumer.

At the time of this migration, a fresh clone or CI runner could not restore the
package because only the cache carried it. That historical limitation is now
addressed by the anonymous static feed described in
[`release-governance.md`](release-governance.md). Consumers use the canonical
source mapping in [`platform-consumer-adoption.md`](platform-consumer-adoption.md);
GitHub Packages and consumer credentials are not part of the current design.
