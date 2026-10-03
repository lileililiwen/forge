# Tasks: Shared MSBuild policy as a packed `Platform.Build` package

Phases are BFS → DFS → BFS. Part 1 only: the package, its conformance coverage
and its documentation. Parts 2 (`forge`'s confirmed set) and 3 (the consumer
migration and the root-file retirement) are not implemented here; the
cross-repo work is recorded in
[`docs/platform-build-policy-migration.md`](../../../docs/platform-build-policy-migration.md).

## 0. Caveats and confirmations

- [x] **Read `openspec/specs/platform-consumer-bootstrap/spec.md` and
  `docs/workspace-consumer-bootstrap.md` first.** `build/Platform.Consumer.props:93-99`
  already asserts `Nullable`, `LangVersion`, `AnalysisLevel`,
  `TreatWarningsAsErrors` and `ManagePackageVersionsCentrally`. The new package
  MUST NOT assert those five (design §3, single-ownership rule). Narrow this
  change rather than duplicating them.
  Enforced by `BuildPolicyConformanceTests.Packed_props_asserts_exactly_the_properties_the_package_owns`
  and `..._does_not_travel_on_the_bootstraps_five_properties`.
- [x] **Public API impact stays "none" in the assembly sense.** No
  `Platform.*` public type, method or signature changes and
  `eng/public-api-baseline.txt` is unaffected. The new package identity and the
  properties it contributes are what this change covers. If implementation needs
  a public API change, stop and re-scope — `AGENTS.md` requires stating it.
  `Platform.Build` ships no assembly at all (`IncludeBuildOutput=false`;
  asserted by `Policy_package_carries_no_runtime_assembly`), and
  `scripts/check-public-api.sh` scans six named projects, none of them this one.
- [x] **Do not edit `forge`, any consumer repository, or
  `/home/paul/code/Directory.Build.props`.** They are recorded as cross-repo
  dependencies and as a migration plan. The preceding archived change set the
  same boundary (`2026-10-01-platform-consumer-bootstrap`: "No edits to any
  consumer repository; migration is documented, not performed").
  Verified by `git status`: the only repository touched is this one. `forge` was
  read, never written.
- [x] **The package MUST NOT assert `TargetFramework` or `RootNamespace`.**
  `forge` already writes both into the generated `.csproj`
  (`generate/mod.rs:858-860`, `mod.rs:871`). A shared package asserting a TFM on
  its consumers is heavy-handed.
  Asserted statically and on a resolved `net8.0` project; see
  `The_policy_package_does_not_impose_a_target_framework`.
- [x] **Do not move `PlatformAsSource` / `PlatformPackageVersion` /
  `PlatformConsumerSourceRoot` into the package.** They depend on a local
  filesystem `Exists(...)` probe and vary per machine; a feed-restored artifact
  is the wrong home for them.
  The packed props names none of the three, and says why in its header.
- [x] **Confirm the sequencing constraint before starting phase 2.** `forge`
  promotes a package into the confirmed set only when the frozen evidence fixture
  records at least `CONSUMER_BAR = 2` distinct external consumers
  (`forge/src/kit/registry.rs:24`, `registry.rs:349-360`, fixture
  `registry.rs:38-60`). An unlisted package scores 0 and is **provisional**, not
  confirmed. Therefore phase 3 (migration) must produce real adoptions *before*
  part 2 can complete. Do not attempt to force the confirmed set first.
  Re-measured in `forge`: `CONSUMER_BAR = 2`; `PLATFORM_PACKAGE_EVIDENCE` has no
  `Platform.Build` entry; `consumers_of` returns `None` for it, and
  `classify_package` maps that to `0` → `PackageClass::Provisional`. Recorded in
  the migration document. Part 1 therefore ships the package as provisional, by
  construction.
- [x] **Reconcile the `NoWarn` list against reality.** The workspace root file
  has **no** `NoWarn` list (verified: no match in
  `/home/paul/code/Directory.Build.props`). The justified-list convention is
  real but lives in the shadowing repos (e.g. `smotoox/Directory.Build.props`).
  Derive the package's list from an actual justification set, not from the
  briefed inventory. Left unchecked in 2.3 for this reason.
  Reconciliation done and recorded: the only candidate set in the workspace is
  `smotoox`'s 22-entry list, which is a per-repository deviation rather than
  platform policy, so the package ships no `NoWarn` at all and enforces the
  justification rule for any future entry.
- [ ] **Sequencing note, unresolved.** `AGENTS.md` says one active OpenSpec
  change at a time, but two are already active (`ui-shared-layer-reachability`
  at 60/64 tasks; `shared-test-web-factory` complete but unarchived). This
  change adds a third. Decide with the change owner whether to archive
  `shared-test-web-factory` first; this is a coordination question, not a
  technical blocker, and it is deliberately left open.
  **Still open.** A coordination decision for the change owner; part 1 does not
  resolve it and did not archive any other change. One consequence was paid for
  in this part: the `ui-shared-layer-reachability` tripwire
  `UiDistributionTests.The_package_manifest_and_razor_package_version_are_untouched_by_this_capability`
  pins the manifest package count, so adding a package moved it from 75 to 76.
  See 3.4.
- [x] **Stage only this change directory.** Stage explicit paths; never
  `git add -A`.

## 1. BFS — Baseline and impact coverage

- [x] Add the new project to `Platform.sln`. `scripts/conformance.sh:86` packs
  the **solution** into `tests/Platform.ConsumerConformance/.local-feed`, so a
  project absent from the solution never reaches the feed.
  Added under the `src` solution folder; the conformance pack now emits
  `Platform.Build.0.1.0.nupkg` beside the 76 packages already there.
- [x] Confirm the pack shape for a build-only package: `IncludeBuildOutput=false`
  plus the props file packed to `buildTransitive/`. There is **no source-level
  precedent** for `buildTransitive` / `IncludeBuildOutput` /
  `DevelopmentDependency` in this repository — verified by search over `src/`
  and `templates/`.
  Confirmed: the only `buildTransitive` matches under `src/` and `templates/`
  are in `obj/**.nuget.g.targets` files generated from *Microsoft* packages. The
  one source-level precedent for a content-only pack is
  `templates/Platform.Application.Template`, whose shape this project follows
  (`PackageType`/`IncludeBuildOutput`/`EnableDefaultCompileItems`, `NU5128`
  suppressed). `DevelopmentDependency` was deliberately not added: the spec's
  pack shape does not include it and the props should flow transitively.
  The pack shape that actually works is recorded in the project file: NuGet
  resolves a packed item's target as `PackagePath` **plus** the item's
  `%(RecursiveDir)`, so the source props must sit at the project root to land
  at `buildTransitive/Platform.Build.props`.
- [x] Reconcile the properties against all three current homes so the ownership
  table in design §3 is complete and disjoint: `build/Platform.Consumer.props:93-99`
  (five properties, opt-in), this repository's `Directory.Build.props:3-9`
  and `:12-32` (its own build and packaging defaults), and the workspace root file
  (TFM default + transport switch only).
  All three read. The table is encoded in
  `BuildPolicyConformanceTests.PackageOwnedProperties`,
  `...BootstrapOwnedProperties` and `...ProjectOwnedProperties`, and the
  workspace root file's actual contents are recorded in the migration document
  (TFM default, `PlatformConsumerSourceRoot`, `PlatformAsSource`,
  `PlatformPackageVersion` — and nothing else).
- [x] Inventory the 26 inheriting repositories and record each one's version
  declaration style. Verified: **none** of the 26 has a
  `Directory.Packages.props` at any depth (only `forge`, which has a nested one),
  so a `PackageReference` alone is not sufficient — each repo needs a version
  declaration too, and that style is a per-repo decision.
  **Re-measured: 25, not 26.** See
  [`docs/platform-build-policy-migration.md`](../../../docs/platform-build-policy-migration.md)
  for the full manifest, the rollback manifest, and why the change's own
  "49, not 48" correction does not hold (`forge` is a Rust repository with no
  `.csproj` outside its own `target/`).
- [x] Record the migration manifest and the rollback manifest for phase 3, and
  the verification predicate for "migrated" that gates root-file deletion.
  [`docs/platform-build-policy-migration.md`](../../../docs/platform-build-policy-migration.md)
  holds both manifests and a five-clause deletion precondition. The predicate is
  deliberately conjunctive: all 25 rows migrated **and** each repository's build
  verified with the file absent **and** its build output unchanged **and** no
  repository depending on the transport switch **and** the generator emitting the
  reference.
- [x] Confirm the specs this change extends rather than replaces:
  `platform-consumer-adoption` (packed artifacts, exact pinning, upgrade/
  rollback) and `platform-consumer-bootstrap` (the import-by-path route).
  Both read. `platform-consumer-adoption` gains one added requirement (the
  policy package as the documented route); `platform-consumer-bootstrap` is
  extended by reference and cross-link only — its five properties stay owned by
  `build/Platform.Consumer.props`.

## 2. DFS — Requirement-by-requirement implementation

- [x] Create the `Platform.Build` project: build-only, no runtime assembly, no
  `TargetFramework` assertion on consumers.
  `src/Platform.Build/Platform.Build.csproj`, `net10.0`,
  `IncludeBuildOutput=false`, `EnableDefaultCompileItems=false`, no references.
- [x] Author `buildTransitive/Platform.Build.props` asserting only the properties
  assigned to it in design §4 — `EnforceCodeStyleInBuild`, `Deterministic`,
  `NoWarn`, `CentralPackageTransitivePinningEnabled`, and `ContinuousIntegrationBuild`
  under its CI condition. Each with `Condition="'$(X)' == ''"` so a consumer's
  value wins. Assert none of the five properties owned by
  `Platform.Consumer.props`, and neither `TargetFramework` nor `RootNamespace`.
  `src/Platform.Build/Platform.Build.props` asserts four: `EnforceCodeStyleInBuild`,
  `Deterministic`, `CentralPackageTransitivePinningEnabled` and, under
  `'$(CI)' == 'true'`, `ContinuousIntegrationBuild`. It asserts **no** `NoWarn`
  (see 2.3), none of the bootstrap's five, and neither `TargetFramework` nor
  `RootNamespace`. **Design §4 and the proposal both list
  `TreatWarningsAsErrors` as package content while §3 assigns it to
  `Platform.Consumer.props`; §3 and tasks 0.1/2.2 win, so it is deliberately
  absent. Flagged in the change's own evidence.**
- [ ] Populate the `NoWarn` list with a one-line justification per entry, and
  reject the list if any entry is unjustified. **Deliberately open:** the exact
  entries are not fixed by this change and the workspace root file supplies no
  starting set; the justification set must be decided during implementation.
  **Left unchecked.** The rejection half is implemented
  (`Every_NoWarn_suppression_in_the_packed_props_carries_a_justification`
  over an empty allow-list, so an unjustified entry fails the build); the
  populate half is not done, because the only candidate set in the workspace is
  `smotoox/Directory.Build.props`'s 22 entries, which is one repository's
  deviation rather than platform policy, and adopting it would suppress
  diagnostics this repository does not currently suppress. Shipping no list is
  strictly safer than shipping an inherited one. **Next action:** decide the
  justification set with the change owner, then add entries to
  `JustifiedSuppressions` in `BuildPolicyConformanceTests` in the same commit.
- [x] Pack to the existing local feed at the platform version and confirm the
  `.nupkg` lands in `tests/Platform.ConsumerConformance/.local-feed` beside the
  76 packages already there.
  `scripts/conformance.sh` emits `Platform.Build.0.1.0.nupkg` in that feed.
  Asserted from the archive itself, not the pack log, by
  `Policy_package_packs_its_props_file_into_buildTransitive`; the
  build-only shape is asserted by `Policy_package_carries_no_runtime_assembly`.
- [x] Extend `scripts/conformance.sh` with a fixture that asserts the **resolved
  value** of each owned property. A passing build is not sufficient evidence for
  a build-only package: it restores and builds whether or not its props are
  honoured (spec: *conformance SHALL prove the policy is applied, not merely
  restored*).
  `scripts/conformance.sh` now restores each `Fixtures/BuildPolicy/` project
  from the feed it just wrote and builds four of them;
  `BuildPolicyConformanceTests` asserts the resolved values. The restore step is
  not cosmetic: MSBuild imports the *extracted* copy under the conformance
  project's `.nuget-packages`, so replacing the `.nupkg` without re-restoring
  leaves the previous props in place.
- [x] Add a second fixture that sets one owned property itself and asserts its
  value survives, and one fixture on a non-default target framework asserting the
  package does not change it.
  `Fixtures/BuildPolicy/ConsumerOverride` sets three owned properties in its own
  body and keeps all three; `Fixtures/BuildPolicy/NonDefaultTarget` targets
  `net8.0` and resolves `TargetFramework=net8.0` unchanged while still receiving
  the policy.
- [x] **Verify the import-order property empirically** before relying on it: that
  a `buildTransitive` props file is imported before the project body, and that
  under the workspace root `Directory.Build.targets` the resolved value matches
  the design §3 ownership table. Currently inherited from the change owner's
  measurement, not verified here. Design §3 is written to be safe either way;
  this task converts it from inherited to established.
  Established from both sides, not assumed.
  (a) *The props file is imported before the project body*: the
  `ConsumerOverride` fixture sets `EnforceCodeStyleInBuild=false` and
  `Deterministic=false` in its body and both survive a props file that asserts
  `true`. A props file imported after the body could not produce that result.
  (b) *A later import still lands its own values*: the `SharedOwnership` fixture
  reproduces the workspace arrangement — a `buildTransitive` props file plus a
  trailing `Platform.Consumer.props` import in the position
  `Directory.Build.targets` occupies — and the bootstrap's `Nullable=enable`,
  `LangVersion=latest` and `AnalysisLevel=latest-recommended` resolve even though
  the package was imported first. If the package had claimed any of them behind
  a `== ''` guard, its earlier write would have survived instead.
  (c) *Nothing is asserted twice*: `Packed_props_asserts_exactly_the_properties_the_package_owns`
  reads the packed props and fails if it claims any of the bootstrap's five or any
  project-identity property. Confirmed non-vacuous by mutation — see the change's
  evidence.
  Note on the fixture's construction: the workspace `Directory.Build.targets`
  deliberately excludes projects inside this repository
  (`!StartsWith('$(PlatformConsumerSourceRoot)')`), so the trailing import is
  reproduced explicitly rather than inherited.
- [x] Extend the `platform-consumer-adoption` pinned-version conformance to
  cover the new package: exact version required, floating ranges and wildcards
  rejected.
  `AdoptionConformanceTests.Build_policy_fixtures_pin_every_Platform_package_to_an_exact_version`
  scans every `Fixtures/BuildPolicy/*.csproj` and applies the same
  `ContainsFloating` rule the existing test uses, plus
  `BuildPolicyConformanceTests.Policy_package_is_pinned_to_an_exact_version`. The
  extension is necessary because the main conformance project demonstrates the
  library packages and does not reference `Platform.Build`.
- [x] Update `docs/platform-consumer-adoption.md` to name the policy package the
  supported way to obtain shared MSBuild policy, and cross-link
  `docs/workspace-consumer-bootstrap.md` in both directions so the package route
  and the import-by-path route are distinguishable from either entry point.
  The adoption document gained a "Shared MSBuild policy: the `Platform.Build`
  package" section (what it asserts, what it deliberately does not, the
  single-ownership boundary, and how to verify the policy applied). The bootstrap
  document gained a header stating it describes the import-by-path route only,
  and a note under its consumer-defaults table. Both link to the other. A new
  `docs/platform-build-policy-migration.md` records the migration.

## 3. BFS — Cross-surface regression and completeness

- [x] Re-run restore / build / test / pack for `Platform.sln`.
  `dotnet test Platform.sln -c Release`: 1450 passed, 0 failed — identical to the
  pre-change baseline. `scripts/conformance.sh` passes end to end.
- [x] Verify no drift between the policy package and `Platform.Consumer.props`:
  for every property in the design §3 ownership table, exactly one file asserts
  it. This is the check that fails if the single-ownership rule is violated.
  Enforced on the *packed* props, not the source, and proven non-vacuous: adding
  a `TreatWarningsAsErrors` assertion to the packed props fails three tests,
  including the named one
  ("`TreatWarningsAsErrors` is owned by build/Platform.Consumer.props; the
  policy package must not also assert it`").
- [x] Verify this repository's own build is unchanged. Its
  `Directory.Build.props` sets `LangVersion`, `Nullable`, `ImplicitUsings`,
  `Deterministic`, `TreatWarningsAsErrors`, `AnalysisLevel` and
  `GenerateDocumentationFile` (lines 3-9) and its packaging defaults (lines
  12-32) — none of these may be silently redefined by the new package.
  No project in the solution references `Platform.Build`; the package is consumed
  only by out-of-tree fixtures, so the repository's own evaluation cannot reach
  its props. Confirmed by the unchanged test count and by
  `Platform.Build` appearing nowhere in any project file.
- [x] Run the architecture tests and the public-API baseline check; confirm
  `eng/public-api-baseline.txt` is unchanged.
  `Platform.Architecture.Tests` passes (367 tests).
  `scripts/check-public-api.sh` reports no diff; the baseline file is untouched.
  **One cross-change edit was required and is disclosed here:** the manifest
  package count assertion in
  `UiDistributionTests.The_package_manifest_and_razor_package_version_are_untouched_by_this_capability`
  moved from 75 to 76, and `eng/package-manifest.json` was regenerated by
  `scripts/generate-package-manifest.sh` (76 packages, all at `0.1.0`). That test
  is a tripwire for the *UI* capability's blast radius, not a statement about
  the size of the platform; its `Platform.UI.Razor` assertions are unchanged and
  the comment now says which capability moved the count. No test was weakened.
- [x] **Cross-repo dependency (not edited here) — record in the change's
  evidence.** In `forge`, adding `Platform.Build` to the kit descriptor's
  confirmed set is what makes generated projects both reference and pin it:
  `confirmed_names()` (`forge/src/kit/registry.rs:312-317`) drives the
  `<PackageReference Include="{name}" />` emission at
  `forge/src/generate/mod.rs:866` **and** the
  `<PackageVersion Include="{name}" Version="{version}" />` lines at
  `forge/src/generate/mod.rs:902`; `version_of()` (`registry.rs:327-329`) with
  `platform_versions()` (`registry.rs:435-446`) pins every confirmed package to
  `PLATFORM_PACKAGE_VERSION` (`registry.rs:450`). Confirm the entry is then
  promoted only after the evidence fixture records ≥2 external consumers.
  Re-measured in `forge` during this part: `CONSUMER_BAR = 2`;
  `PLATFORM_PACKAGE_EVIDENCE` has no `Platform.Build` row; `consumers_of` yields
  `None`; `classify_package` maps `None` to `0`, which is below the bar, so the
  package is `Provisional` and stays out of the confirmed set. `forge` was read
  only. Note for part 2: `classify_package`'s own comment says the completeness
  test *names* an unclassified package, so adding a shipped `Platform.*` package
  is expected to surface a gap there. That is part 2's work, not this part's.
- [x] **Cross-repo migration plan (not performed here) — record as reviewable.**
  Each of the 26 inheriting repositories adds one `PackageReference` plus a
  version declaration (0 of 26 currently has a `Directory.Packages.props`, so
  CPM adoption is a per-repo choice, not a given). Ship the package and migrate
  repos first; only then re-measure the `forge` evidence fixture and promote.
  Recorded in
  [`docs/platform-build-policy-migration.md`](../../../docs/platform-build-policy-migration.md):
  a 25-row manifest, a rollback manifest, and the per-repository version-style
  decision with a recommendation. Not performed: no consumer repository was
  touched. The row count is 25, not 26 — see 1.4.
- [x] **Record the precondition for deleting
  `/home/paul/code/Directory.Build.props`:** every one of the 26 is verified
  migrated, and a restore from a clone without the file succeeds. The file MUST
  NOT be deleted while any consumer remains — a live consumer of a deleted
  baseline fails worse than a duplicated one, because the failure is a missing
  target framework rather than a divergent policy.
  Recorded as a five-clause conjunction in the migration document, and
  restated as "the file must not be deleted while any consumer remains" with the
  reason. Clause 5 blocks deletion on the generator emitting the reference, so
  deletion cannot precede part 2.
- [x] Confirm the recorded decision holds: the 23 repositories that already ship
  their own `Directory.Build.props` are **out of scope and left untouched**.
  They are self-sufficient, and migrating them would mean reviewing 23
  independent local baselines — several with documented reasons for shadowing —
  for no benefit. The accepted cost is that shared policy is not uniform across
  the workspace; state this rather than hide it.
  Re-measured: 23 shadow, and `therapist-commons`, `citylens`, `devloom`,
  `kairion`, `mewo` and `smotoox` were each confirmed to carry their own file.
  None was edited. The accepted non-uniformity is stated in the migration
  document rather than hidden.
- [x] `git diff --check` and strict OpenSpec validation pass before completion.

## 4. Verification

- [x] `dotnet restore && dotnet build Platform.sln -c Release && dotnet test
  Platform.sln -c Release && dotnet pack Platform.sln -c Release -o
  artifacts/packages`.
  Run as `dotnet test Platform.sln -c Release` (1450 passed, 0 failed) and
  `bash scripts/conformance.sh`, whose pack step is `dotnet pack` over the
  solution. `artifacts/packages` was not used as the output: the feed the
  conformance suite reads is `tests/Platform.ConsumerConformance/.local-feed`,
  which is what 2.4 asserts against.
- [x] `./scripts/conformance.sh`, including the new resolved-property fixtures.
- [ ] `./scripts/consumer-upgrade-rollback.sh` for the new package, proving
  candidate-then-previous restore.
  **Left unchecked — the script does not run, for reasons that predate this
  change.** Exact failing command:
  `bash scripts/consumer-upgrade-rollback.sh tests/Platform.ConsumerConformance/.local-feed tests/Platform.ConsumerConformance/.local-feed 0.1.0`
  Three defects, each confirmed by isolating it:
  1. `tar -C "$work_dir" -xf -` fails — `tar: /tmp/tmp.XXXX/fixture: cannot
     open: No such file or directory` — because `work_dir="$temp_dir/fixture"`
     is never created. The script dies before restoring anything.
  2. Reading the manifest then fails —
     `FileNotFoundError: '/tmp/tmp.XXXX/fixture/../../eng/package-manifest.json'`
     — because `../../` from `$temp_dir/fixture` resolves to `/tmp`, not to a
     temp copy of the repository's `eng/`. Neither this script nor the suite's
     `AdoptionConformanceTests` can find the manifest in the temp tree.
  3. With 1 and 2 worked around locally, the fixture run fails 10 path-dependent
     tests, because the suite locates the platform repository root by walking up
     from `AppContext.BaseDirectory` for `build/Platform.Consumer.props`
     (`BootstrapConformanceTests.LocatePlatformRepositoryRoot`) and the temp tree
     contains neither `build/` nor `.local-feed/`. The new
     `BuildPolicyConformanceTests` uses the same root-walking convention, so it
     would fail there too.
  **Not fixed here, deliberately.** Making the rehearsal actually pass requires
  the conformance suite to take the feed and repository root as inputs rather
  than discovering them by walking up from the assembly, which is a change to
  the suite's test harness and outside part 1's edit surface. Half-fixing the
  script would leave it exiting non-zero with the real blocker still hidden.
  **Next action:** land a follow-up that parameterises the suite's root and feed
  lookup (for example via an environment variable read by
  `LocatePlatformRepositoryRoot`), then re-run this task. The new package is not
  otherwise unrehearsed: `Policy_package_is_pinned_to_an_exact_version` and
  `Policy_package_ships_at_the_platform_version_alongside_the_others` assert the
  exact-version and single-version-set properties the rehearsal would check.
- [x] `./scripts/quality-gate.sh`, `./scripts/check-public-api.sh`,
  `git diff --check`, and `openspec validate platform-build-policy-package --strict`.
- [x] `openspec list --changes` to confirm the change is listed. Note:
  `openspec list --changes --strict` is **not** valid on the pinned CLI v1.6.0;
  use `openspec list --changes` and `openspec validate --changes --strict`.
- [x] Record actual outputs. Report any environment-blocked check with its exact
  failing command and next action rather than as passing. Leave any unresolved
  item unchecked with its reason.
