# Proposal: Shared MSBuild policy as a packed `Platform.Build` package

## Why

The workspace MSBuild baseline is unversioned, unowned by any commit, and
discovered only by the upward directory walk:

- `/home/paul/code/Directory.Build.props` is not in a Git repository. The
  workspace root has no `.git` (`git -C /home/paul/code rev-parse` reports
  "not a git repository"), so no commit records the file, no review sees it, and
  no history explains it. MSBuild finds it by walking up, so **49 .NET Git
  repositories** in this workspace inherit it implicitly (verified: 23 ship
  their own root `Directory.Build.props`; the remaining 26 inherit this one).
- `forge` emits no `Directory.Build.props`. The `aspnet-web` generator writes
  `.csproj`, `Program.cs`, `appsettings.json`, `Dockerfile`, `.gitignore`, and —
  only when a kit is present — `NuGet.config` and `Directory.Packages.props`
  (`forge/src/generate/mod.rs:860-923`). The single `Directory.Build.props`
  string in that file is at `mod.rs:442`, inside `is_manifest_path` — a
  dependency-scanning allowlist, not an emitter. So **every new scaffold also
  inherits the unversioned baseline**, and the gap is structural, not a
  one-time oversight.
- The shadowed repos mostly shadow silently. 23 repositories ship their own
  `Directory.Build.props` that hides the workspace file from their builds. At
  least 5 document why (`therapist-commons`, `citylens`, `devloom`, `kairion`,
  `mewo`); the rest do not. `devloom` and `kairion` document the same specific
  escape — refusing to inherit the ambient `PlatformAsSource` ProjectReference
  injection — which is evidence the inheritance is actively harmful, not merely
  untidy.

The transport that would fix this is already proven. `triplanify`
(`nuget-feed/Platform.{Core,AspNetCore,Testing}.0.1.0.nupkg`) and
`therapist-commons` (`.packages/local-feed/`, exact `0.1.0` `PackageVersion`
lines) consume packed platform artifacts from committed local feeds, under the
exact-version pinning policy in `docs/platform-consumer-adoption.md`.

## What Changes

1. **This repository** — add a build-only package `Platform.Build` that ships
   `buildTransitive/Platform.Build.props` carrying the shared MSBuild policy
   (warnings-as-errors, code-style enforcement, deterministic output, the
   justified `NoWarn` list, central package management, and the CI flags), packed
   into the existing `tests/Platform.ConsumerConformance/.local-feed` at an exact
   version, versioned, and reachable through `scripts/conformance.sh`.
   `docs/platform-consumer-adoption.md` becomes the document that names this the
   supported way to obtain shared MSBuild policy.
2. **Cross-repo dependency in `forge`** — add `Platform.Build` to the kit
   descriptor's confirmed set so generated projects both reference and pin it.
   `forge` is **not** edited by this change; it is recorded as a dependency with
   the file:line evidence.
3. **Cross-repo migration** — the 26 inheriting repositories each add one
   `PackageReference`, after which the workspace root file has no consumer. This
   is recorded as a reviewable plan with a rollback manifest, and with an
   explicit precondition for deleting the root file.

## The decision: a packed props package, not a template and not a copied file

The policy ships as a NuGet `buildTransitive/` props file inside a package in
this repository. The alternatives were rejected for recorded reasons:

- **A forge template is copied text.** It would produce 25+ frozen snapshots
  that drift the moment the policy changes, and no copy records which version a
  project was built against. That is exactly the duplication
  `docs/platform-consumer-adoption.md` exists to prevent.
- **A copied root file** is the status quo and is the thing being removed: it is
  unowned by any commit and discovered only by an implicit walk.
- **A `buildTransitive/` package** is versioned, centrally owned, and applies
  automatically to every project with a `PackageReference` — zero per-repo
  copies. The transport is already proven by the two existing consumers.

## Scope boundary: the package owns policy, not the target framework

`TreatWarningsAsErrors`, `EnforceCodeStyleInBuild`, `Deterministic`, the
justified `NoWarn` list, `ManagePackageVersionsCentrally`, and the CI
(`ContinuousIntegrationBuild`) flags **belong in the package**.

`TargetFramework` and `RootNamespace` **do not**. A shared package asserting a
TFM on its consumers is heavy-handed, and `forge` already writes both into the
generated `.csproj` (`mod.rs:858-860` takes the TFM from the kit descriptor;
`mod.rs:871` writes it alongside `RootNamespace`). MSBuild imports
`buildTransitive` props *before* the project body, so a consumer can still
override a TFM the package asserts — the package simply should not assert one.
This import-order property is inherited from the change owner's measurement and
is listed as a task to verify empirically before it is relied on.

## Public API impact

**None in the assembly sense.** No `Platform.*` public type, method or signature
changes, and `eng/public-api-baseline.txt` is unaffected. The change adds one
new package identity (`Platform.Build`) and the MSBuild properties it
contributes. `AGENTS.md` requires public API changes to be covered by an
OpenSpec change, and this change covers the new package boundary it introduces.

## Corrections to the briefed inventory

Recorded because the spec must not rest on a wrong reading of the current file.
All figures below were re-measured in this session.

- **The root file does not contain the described policy.** The brief listed 13
  items as its current contents. `/home/paul/code/Directory.Build.props` (1271
  bytes) contains only: a guarded `TargetFramework` default of `net10.0`,
  `PlatformConsumerSourceRoot`, `PlatformAsSource` (two guarded conditions), and
  `PlatformPackageVersion`. A direct search for `ManagePackageVersionsCentrally`,
  `CentralPackageTransitivePinningEnabled` and `EnforceCodeStyleInBuild` in that
  file returns no matches. There is **no** `NoWarn` list, no `Nullable`, no
  `TreatWarningsAsErrors`, no `Deterministic`, no `AnalysisLevel`, no
  `LangVersion`, no `ImplicitUsings`, and no `ContinuousIntegrationBuild`
  condition in it.
- **That policy already exists in this repository, in two other places.**
  `build/Platform.Consumer.props:94-98` sets `Nullable`, `LangVersion`,
  `AnalysisLevel`, `TreatWarningsAsErrors` and `ManagePackageVersionsCentrally`
  (each guarded by `Condition="'$(X)' == ''"`), behind the
  `PlatformConsumerBootstrap=true` opt-in. This repository's own
  `Directory.Build.props:3-9` sets `LangVersion`, `Nullable`, `ImplicitUsings`,
  `Deterministic`, `TreatWarningsAsErrors`, `AnalysisLevel` and
  `GenerateDocumentationFile`. The justified-`NoWarn` convention is real but
  lives in the shadowing repos (e.g. `smotoox/Directory.Build.props` requires
  "each entry MUST carry a one-line justification"), not in the root file.
- **The count is 49, not 48.** 23 ship their own `Directory.Build.props`; 26
  inherit the workspace one. The briefed 25-name list is exactly correct, but
  `forge` is itself a .NET repository that also inherits the root file and was
  in neither bucket. This strengthens the case: the generator is a consumer of
  the very baseline it does not emit.
- **More than two repos document their shadowing.** The brief said only
  `therapist-commons` and `citylens` do; `devloom`, `kairion` and `mewo` also
  carry explanatory comments.
- **Migration cost is one line per repo more than briefed.** None of the 26
  inheriting repositories has a `Directory.Packages.props` at any depth (only
  `forge`, which has a nested one). So each migrated repo must add a
  `PackageReference` **and** a version declaration, and the version-declaration
  style is a per-repo decision.

## BFS Impact Map

- **Capabilities:** a new `platform-build-policy` capability; a new
  requirement in `platform-consumer-adoption` naming the package as the supported
  route to shared MSBuild policy.
- **Owned here:** the new package project, its `buildTransitive` props, its
  packing into `.local-feed`, the conformance fixture that proves the props
  apply, and the documentation update.
- **Cross-repo dependencies (recorded, not edited):** the `forge` kit
  descriptor's confirmed set; the 26 inheriting consumer repositories.
- **Compatibility:** repos that already shadow the workspace file are
  unaffected — see the recorded scope decision below.
- **Conflict risk:** `build/Platform.Consumer.props` already sets five of the
  same properties. See design §3; single ownership is a requirement, not a
  preference.

## Recorded scope decision: the 23 already-self-sufficient repositories stay as they are

**Decision: out of scope, left alone.** They ship their own
`Directory.Build.props`, so they are already self-sufficient and the workspace
file does not reach their builds. Reason: migrating them would mean reviewing 23
independent local baselines — each with its own deliberate deviations and, in
several cases, documented reasons for shadowing — for a benefit they do not
need. It would also convert a silent-but-harmless arrangement into 23 reviewed
diffs, which is scope the problem does not justify. The cost of leaving them is
that the shared policy is *not* uniform across the workspace, which this change
accepts and states rather than hides.

## Capabilities

- `platform-build-policy` (new).
- `platform-consumer-adoption` (one added requirement: shared MSBuild policy is
  obtained from the packed package).

## Non-goals

- No production implementation in this change. This is a spec.
- No edit to `forge`, to any consumer repository, or to
  `/home/paul/code/Directory.Build.props`. Those are recorded as cross-repo
  dependencies and as a migration plan.
- No deletion of the workspace root file in this change. Deletion is gated on a
  verified migration precondition.
- No change to `Platform.Core` or to any existing package's public API.
- No new package runtime/assembly code: `Platform.Build` carries MSBuild policy,
  not a library.
- No replacement of `build/Platform.Consumer.props`. It keeps its own path; the
  design states how the two do not drift.

## Package Boundary and Split Assessment

**Single outcome:** shared MSBuild policy reaches a consuming project through a
versioned, centrally owned, packed artifact instead of an unowned file found by
a directory walk.

**Included:** the `Platform.Build` package, its props content and its
deliberate exclusions, its packing and conformance coverage, and the
documentation that names it the supported route.

**Excluded:** the forge descriptor edit, the 26-repository migration, and the
root-file deletion — each is a cross-repo dependency with a named owner and a
recorded precondition.

**Split signals considered:** the three parts share one owner (this repository's
release), one lifecycle (the platform version), and one oracle (a fixture that
restores the packed package and asserts the properties resolve). Splitting the
package from the forge wiring would ship a package nothing references; splitting
the migration from the package would leave the root file with no deletion
precondition. They stay one change with three scoped parts.

**Ordering is forced, not chosen.** `forge` only promotes a package into the
confirmed set when a frozen evidence fixture records at least
`CONSUMER_BAR = 2` distinct external consumers
(`forge/src/kit/registry.rs:24`, `registry.rs:349-360`, fixture at
`registry.rs:38-60`); an unlisted package scores 0 and is **provisional**, not
confirmed. So `Platform.Build` cannot enter the confirmed set until real
repositories have adopted it. The phases below are sequenced to respect that.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `build/Platform.Consumer.props` | `build/Platform.Consumer.props:93-99` | already sets `Nullable`, `LangVersion`, `AnalysisLevel`, `TreatWarningsAsErrors`, `ManagePackageVersionsCentrally`, each `== ''` guarded | requires an **import by path** and the `PlatformConsumerBootstrap=true` opt-in; not restore-driven | this repository | **extend, do not replace**; design §3 assigns single ownership so the two cannot drift |
| `platform-consumer-bootstrap` spec | `openspec/specs/platform-consumer-bootstrap/spec.md:31-50` | asserts the defaults propagate with an opt-out | describes the import-by-path route only | this repository | **adopt**; the new package is the restore-driven sibling, and the shared-ownership rule is stated in the new capability |
| `platform-consumer-adoption` spec | `openspec/specs/platform-consumer-adoption/spec.md` | packed-artifact conformance, exact-version pinning, upgrade/rollback | silent on where shared MSBuild policy comes from | this repository | **extend** with one added requirement |
| `docs/platform-consumer-adoption.md` | `:37-50` | exact-version pinning; rejects floating ranges and wildcards | names no source of shared build policy | this repository | **adopt and extend** — it becomes the governing document for this route |
| `scripts/conformance.sh` + `.local-feed` | `scripts/conformance.sh:21,86`; `.local-feed` holds 76 `Platform.*.0.1.0.nupkg` | pack → restore → build → test against one local feed | the package is build-only, so a restore alone proves nothing about props application | this repository | **extend** with a fixture that asserts the properties resolve |
| `forge` kit descriptor | `forge/src/kit/registry.rs:312-317`, `generate/mod.rs:866,902` | `confirmed_names()` already drives both the `<PackageReference>` and the `<PackageVersion>` | confirmed set is gated on a frozen consumer-count fixture | `forge` | **cross-repo dependency; do not edit** |
| workspace root `Directory.Build.props` | `/home/paul/code/Directory.Build.props` (outside this repository) | the switch the change retires | lives outside version control entirely | workspace governance | **not edited here**; deletion is gated on a verified precondition |
| 23 shadowing repositories | e.g. `therapist-commons`, `devloom` | self-sufficient local baselines | shared policy will not be uniform | each repo's owner | **out of scope** — recorded decision above |

## Evidence boundary

This change can prove, once implemented, that the packed `Platform.Build`
package restores from the committed feed, that its props are applied to a
consuming project, that the properties it asserts match the recorded policy, and
that a consumer can still override each one. It cannot prove that any of the 26
consumer repositories has actually migrated — that is the cross-repo migration
plan's evidence, not this change's. It also cannot prove the `buildTransitive`
import-order property without the empirical fixture, which is why that
verification is an explicit unchecked task rather than a stated fact.
