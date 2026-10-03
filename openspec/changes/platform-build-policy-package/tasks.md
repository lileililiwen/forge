# Tasks: Shared MSBuild policy as a packed `Platform.Build` package

All boxes are unchecked. This change is a specification; no implementation has
landed. Phases are BFS → DFS → BFS.

## 0. Caveats and confirmations

- [ ] **Read `openspec/specs/platform-consumer-bootstrap/spec.md` and
  `docs/workspace-consumer-bootstrap.md` first.** `build/Platform.Consumer.props:93-99`
  already asserts `Nullable`, `LangVersion`, `AnalysisLevel`,
  `TreatWarningsAsErrors` and `ManagePackageVersionsCentrally`. The new package
  MUST NOT assert those five (design §3, single-ownership rule). Narrow this
  change rather than duplicating them.
- [ ] **Public API impact stays "none" in the assembly sense.** No
  `Platform.*` public type, method or signature changes and
  `eng/public-api-baseline.txt` is unaffected. The new package identity and the
  properties it contributes are what this change covers. If implementation needs
  a public API change, stop and re-scope — `AGENTS.md` requires stating it.
- [ ] **Do not edit `forge`, any consumer repository, or
  `/home/paul/code/Directory.Build.props`.** They are recorded as cross-repo
  dependencies and as a migration plan. The preceding archived change set the
  same boundary (`2026-10-01-platform-consumer-bootstrap`: "No edits to any
  consumer repository; migration is documented, not performed").
- [ ] **The package MUST NOT assert `TargetFramework` or `RootNamespace`.**
  `forge` already writes both into the generated `.csproj`
  (`generate/mod.rs:858-860`, `mod.rs:871`). A shared package asserting a TFM on
  its consumers is heavy-handed.
- [ ] **Do not move `PlatformAsSource` / `PlatformPackageVersion` /
  `PlatformConsumerSourceRoot` into the package.** They depend on a local
  filesystem `Exists(...)` probe and vary per machine; a feed-restored artifact
  is the wrong home for them.
- [ ] **Confirm the sequencing constraint before starting phase 2.** `forge`
  promotes a package into the confirmed set only when the frozen evidence fixture
  records at least `CONSUMER_BAR = 2` distinct external consumers
  (`forge/src/kit/registry.rs:24`, `registry.rs:349-360`, fixture
  `registry.rs:38-60`). An unlisted package scores 0 and is **provisional**, not
  confirmed. Therefore phase 3 (migration) must produce real adoptions *before*
  part 2 can complete. Do not attempt to force the confirmed set first.
- [ ] **Reconcile the `NoWarn` list against reality.** The workspace root file
  has **no** `NoWarn` list (verified: no match in
  `/home/paul/code/Directory.Build.props`). The justified-list convention is
  real but lives in the shadowing repos (e.g. `smotoox/Directory.Build.props`).
  Derive the package's list from an actual justification set, not from the
  briefed inventory. Left unchecked in 2.5 for this reason.
- [ ] **Sequencing note, unresolved.** `AGENTS.md` says one active OpenSpec
  change at a time, but two are already active (`ui-shared-layer-reachability`
  at 60/64 tasks; `shared-test-web-factory` complete but unarchived). This
  change adds a third. Decide with the change owner whether to archive
  `shared-test-web-factory` first; this is a coordination question, not a
  technical blocker, and it is deliberately left open.
- [ ] **Stage only this change directory.** Stage explicit paths; never
  `git add -A`.

## 1. BFS — Baseline and impact coverage

- [ ] Add the new project to `Platform.sln`. `scripts/conformance.sh:86` packs
  the **solution** into `tests/Platform.ConsumerConformance/.local-feed`, so a
  project absent from the solution never reaches the feed.
- [ ] Confirm the pack shape for a build-only package: `IncludeBuildOutput=false`
  plus the props file packed to `buildTransitive/`. There is **no source-level
  precedent** for `buildTransitive` / `IncludeBuildOutput` /
  `DevelopmentDependency` in this repository — verified by search over `src/`
  and `templates/`.
- [ ] Reconcile the properties against all three current homes so the ownership
  table in design §3 is complete and disjoint: `build/Platform.Consumer.props:93-99`
  (five properties, opt-in), this repository's `Directory.Build.props:3-9` and
  `:12-32` (its own build and packaging defaults), and the workspace root file
  (TFM default + transport switch only).
- [ ] Inventory the 26 inheriting repositories and record each one's version
  declaration style. Verified: **none** of the 26 has a
  `Directory.Packages.props` at any depth (only `forge`, which has a nested one),
  so a `PackageReference` alone is not sufficient — each repo needs a version
  declaration too, and that style is a per-repo decision.
- [ ] Record the migration manifest and the rollback manifest for phase 3, and
  the verification predicate for "migrated" that gates root-file deletion.
- [ ] Confirm the specs this change extends rather than replaces:
  `platform-consumer-adoption` (packed artifacts, exact pinning, upgrade/
  rollback) and `platform-consumer-bootstrap` (the import-by-path route).

## 2. DFS — Requirement-by-requirement implementation

- [ ] Create the `Platform.Build` project: build-only, no runtime assembly, no
  `TargetFramework` assertion on consumers.
- [ ] Author `buildTransitive/Platform.Build.props` asserting only the properties
  assigned to it in design §4 — `EnforceCodeStyleInBuild`, `Deterministic`,
  `NoWarn`, `CentralPackageTransitivePinningEnabled`, and `ContinuousIntegrationBuild`
  under its CI condition. Each with `Condition="'$(X)' == ''"` so a consumer's
  value wins. Assert none of the five properties owned by
  `Platform.Consumer.props`, and neither `TargetFramework` nor `RootNamespace`.
- [ ] Populate the `NoWarn` list with a one-line justification per entry, and
  reject the list if any entry is unjustified. **Deliberately open:** the exact
  entries are not fixed by this change and the workspace root file supplies no
  starting set; the justification set must be decided during implementation.
- [ ] Pack to the existing local feed at the platform version and confirm the
  `.nupkg` lands in `tests/Platform.ConsumerConformance/.local-feed` beside the
  76 packages already there.
- [ ] Extend `scripts/conformance.sh` with a fixture that asserts the **resolved
  value** of each owned property. A passing build is not sufficient evidence for
  a build-only package: it restores and builds whether or not its props are
  honoured (spec: *conformance SHALL prove the policy is applied, not merely
  restored*).
- [ ] Add a second fixture that sets one owned property itself and asserts its
  value survives, and one fixture on a non-default target framework asserting the
  package does not change it.
- [ ] **Verify the import-order property empirically** before relying on it: that
  a `buildTransitive` props file is imported before the project body, and that
  under the workspace root `Directory.Build.targets` the resolved value matches
  the design §3 ownership table. Currently inherited from the change owner's
  measurement, not verified here. Design §3 is written to be safe either way;
  this task converts it from inherited to established.
- [ ] Extend the `platform-consumer-adoption` pinned-version conformance to
  cover the new package: exact version required, floating ranges and wildcards
  rejected.
- [ ] Update `docs/platform-consumer-adoption.md` to name the policy package the
  supported way to obtain shared MSBuild policy, and cross-link
  `docs/workspace-consumer-bootstrap.md` in both directions so the package route
  and the import-by-path route are distinguishable from either entry point.

## 3. BFS — Cross-surface regression and completeness

- [ ] Re-run restore / build / test / pack for `Platform.sln`.
- [ ] Verify no drift between the policy package and `Platform.Consumer.props`:
  for every property in the design §3 ownership table, exactly one file asserts
  it. This is the check that fails if the single-ownership rule is violated.
- [ ] Verify this repository's own build is unchanged. Its
  `Directory.Build.props` sets `LangVersion`, `Nullable`, `ImplicitUsings`,
  `Deterministic`, `TreatWarningsAsErrors`, `AnalysisLevel` and
  `GenerateDocumentationFile` (lines 3-9) and its packaging defaults (lines
  12-32) — none of these may be silently redefined by the new package.
- [ ] Run the architecture tests and the public-API baseline check; confirm
  `eng/public-api-baseline.txt` is unchanged.
- [ ] **Cross-repo dependency (not edited here) — record in the change's
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
- [ ] **Cross-repo migration plan (not performed here) — record as reviewable.**
  Each of the 26 inheriting repositories adds one `PackageReference` plus a
  version declaration (0 of 26 currently has a `Directory.Packages.props`, so
  CPM adoption is a per-repo choice, not a given). Ship the package and migrate
  repos first; only then re-measure the `forge` evidence fixture and promote.
- [ ] **Record the precondition for deleting
  `/home/paul/code/Directory.Build.props`:** every one of the 26 is verified
  migrated, and a restore from a clone without the file succeeds. The file MUST
  NOT be deleted while any consumer remains — a live consumer of a deleted
  baseline fails worse than a duplicated one, because the failure is a missing
  target framework rather than a divergent policy.
- [ ] Confirm the recorded decision holds: the 23 repositories that already ship
  their own `Directory.Build.props` are **out of scope and left untouched**.
  They are self-sufficient, and migrating them would mean reviewing 23
  independent local baselines — several with documented reasons for shadowing —
  for no benefit. The accepted cost is that shared policy is not uniform across
  the workspace; state this rather than hide it.
- [ ] `git diff --check` and strict OpenSpec validation pass before completion.

## 4. Verification

- [ ] `dotnet restore && dotnet build Platform.sln -c Release && dotnet test
  Platform.sln -c Release && dotnet pack Platform.sln -c Release -o
  artifacts/packages`.
- [ ] `./scripts/conformance.sh`, including the new resolved-property fixtures.
- [ ] `./scripts/consumer-upgrade-rollback.sh` for the new package, proving
  candidate-then-previous restore.
- [ ] `./scripts/quality-gate.sh`, `./scripts/check-public-api.sh`,
  `git diff --check`, and `openspec validate platform-build-policy-package --strict`.
- [ ] `openspec list --changes` to confirm the change is listed. Note:
  `openspec list --changes --strict` is **not** valid on the pinned CLI v1.6.0;
  use `openspec list --changes` and `openspec validate --changes --strict`.
- [ ] Record actual outputs. Report any environment-blocked check with its exact
  failing command and next action rather than as passing. Leave any unresolved
  item unchecked with its reason.
