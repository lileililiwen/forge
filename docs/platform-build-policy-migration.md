# Shared MSBuild policy: migration and root-file retirement

The migration plan for the `Platform.Build` policy package, and the
precondition under which the unversioned workspace baseline may be deleted.

**This document is a plan. Nothing here has been performed.** `Platform.Build`
ships the policy; the consumer repositories have not migrated. The evidence for
each repository's state below was measured on 2026-10-03 by walking
`/home/paul/code` for Git repositories containing at least one `*.csproj` and
comparing each one against `/home/paul/code/Directory.Build.props`.

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
at any depth**, so central package management is not available to any of them
today. A migrated repository therefore needs **two** edits, not one: the
`PackageReference` and a version declaration. The version-declaration style is a
per-repository decision, and the cheapest one that satisfies
[`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md)'s
exact-version rule without introducing a new central file is an inline
`Version` attribute on the `PackageReference` itself:

```xml
<PackageReference Include="Platform.Build" Version="0.1.0" />
```

Introducing `Directory.Packages.props` in a repository that has never had one
is a larger decision than this migration: it switches on central package
management for every reference in that repository, not just this one. Where a
repository already wants CPM, `<PackageVersion Include="Platform.Build"
Version="0.1.0" />` is equivalent and preferred. Both forms are exact; neither
uses a range or a wildcard.

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

| # | Repository | .NET projects | Version style today | Migrated |
|---|---|---|---|---|
| 1 | aegify | 2 | none — inline `Version` required | no |
| 2 | aesthra | 2 | none — inline `Version` required | no |
| 3 | arivio | 6 | none — inline `Version` required | no |
| 4 | bargora | 2 | none — inline `Version` required | no |
| 5 | campusMesh | 20 | none — inline `Version` required | no |
| 6 | chinago | 2 | none — inline `Version` required | no |
| 7 | crossify | 2 | none — inline `Version` required | no |
| 8 | cvunify | 2 | none — inline `Version` required | no |
| 9 | fotofy | 16 | none — inline `Version` required | no |
| 10 | hestiaLab | 2 | none — inline `Version` required | no |
| 11 | hypora | 2 | none — inline `Version` required | no |
| 12 | kairosa | 2 | none — inline `Version` required | no |
| 13 | lexora | 6 | none — inline `Version` required | no |
| 14 | linglitch | 2 | none — inline `Version` required | no |
| 15 | mortalect | 5 | none — inline `Version` required | no |
| 16 | namevia | 5 | none — inline `Version` required | no |
| 17 | orphevia | 5 | none — inline `Version` required | no |
| 18 | pangoman | 2 | none — inline `Version` required | no |
| 19 | ploutify | 4 | none — inline `Version` required | no |
| 20 | reelio | 2 | none — inline `Version` required | no |
| 21 | retakeKit | 4 | none — inline `Version` required | no |
| 22 | rostera | 6 | none — inline `Version` required | no |
| 23 | singleatee | 8 | none — inline `Version` required | no |
| 24 | sutravia | 3 | none — inline `Version` required | no |
| 25 | triplanify | 2 | none — inline `Version` required | no |

Two repositories in this table already consume packed platform artifacts
(`triplanify` and `therapist-commons`; only `triplanify` is on this list), which
is why the feed transport is proven rather than assumed.

## Rollback manifest

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
