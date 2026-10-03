# Tasks: scaffold-prewires-shared-layer

Implemented on owner direction. The owner resolved open decision 3 (the
target-framework blocker) by ruling the workspace .NET 10 and removing SDK 8;
the remaining three decisions were made as the least-surprising option and are
recorded with their rationale in `design.md` §12. This package still does not
authorize archive, release or a commit: `openspec archive` is not run, and no
git operation was performed.

**Amended on owner direction, a second time.** The owner rejected the
environment-variable feed mechanism the first implementation used
(`NUGET_PLATFORM_FEED` + `RestoreAdditionalProjectSources`) and stated the
requirement it fails: *a project must not rely on a physical project path, and
the concrete project must still build when it does not reside on that machine —
a CI runner being the named case.* A variable a CI runner does not carry is the
same failure class as a hard-coded absolute path. §6 records the amendment; the
rejected mechanism, its constant, its MSBuild block and the comments justifying
it were removed rather than left dormant.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every existing requirement in `proposal.md` onto the code that
  owns it today: `src/profile/mod.rs` `ProfileDescriptor` (the additive `kit`
  field site), `src/generate/mod.rs` `template_files` and `CreationRequest`
  (the render sites and the `aspnet-web` / `react-web` / `nextjs-web` arms at
  `src/generate/mod.rs:559-591`, `:496-557` and `:470-495`),
  `src/generate/workspace.rs` (the existing staged-ownership pattern the kit
  receipt must mirror), `src/registry/mod.rs` (the project record the pinned kit
  id/version is added to), `src/core/mod.rs` (the typed-error enum and its
  `code()` mapping) and `src/main.rs` (the `New` command and
  `forge profile inspect`).
- [x] 1.2 Confirm the product boundary is preserved: a scaffolded project still
  builds and operates through `dotnet build` / `npm run build` / `cargo build` /
  `flutter analyze` with Forge absent, and `react-web`'s offline,
  dependency-free `npm run build` and `npm test` contract is unchanged. Record
  the exact test that proves each, before implementation, so a later regression
  has a known owner.
- [x] 1.3 Build the evidence table the floor depends on and freeze it as a
  checked-in fixture: the per-package distinct external consumer counts
  (`Platform.Core` 7, `Platform.AspNetCore` 6, `Platform.Testing` 4,
  `Platform.Identity.AspNetCore` 3, `Platform.Persistence.EfCore` 4,
  `Platform.RateLimiting` 2, `Platform.Idempotency` 2, `Platform.Observability`
  2, `Platform.Web.Composition` 1, `Platform.Web.Telemetry` 1,
  `Platform.Testing.AspNetCore` 1, `Platform.UI.Razor` 0,
  `Platform.Http.Resilience` 0, `Platform.FeatureManagement` 0,
  `Platform.Tenant.Lifecycle.AspNetCore` 0) and the per-package
  infrastructure weight (each confirmed member's only project references are
  `Platform.Core` and `Platform.Web.Telemetry`). A test fails when a package's
  classification disagrees with this fixture.
- [x] 1.4 Establish the `kits/` vendored-asset tree and its
  `kits/manifest.json` before any code depends on it, modelled on
  `contracts/manifest.json` in `platform-contract-consumption`: record the
  source revision and the sha256 of every vendored file, verify the digests
  offline with no sibling checkout and no network, and vendor the
  *generated* `tokens.css` / `tokens.ts` and the kit verifier — not the DTCG
  source file, which stays upstream-owned.
- [x] 1.5 Add the test skeletons the implementation will fill: per-profile
  render fixtures, floor met / unmet / exception / exception-without-reason
  cases, feed-value rejection cases, digest-agreement and registry-completeness
  tests, and a controlled-`PATH` no-network assertion for the generation path.
- [x] 1.6 Record the blocker found during reconnaissance — every
  `dotnet-platform-libs` package targets `net10.0` while the `aspnet-web`
  scaffold targets `net8.0`, so the confirmed set cannot restore as declared —
  and put the decision (Forge raises the profile TFM, or the library multi-targets
  down) in front of the owner before §2.4 starts. A `net8.0` manifest that
  silently fails to restore is a worse outcome than the current zero.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the `KitReference` / `FeedRef` / `AssetRef` descriptors and the
  compiled-in kit registry in `src/kit/registry.rs`, with
  `KIT_CONTRACT_VERSION`, offline lookup by `(id, version)`, and the
  confirmed/provisional classification sourced from the §1.3 fixture rather
  than from a hand-maintained duplicate list.
- [x] 2.2 Add the additive `#[serde(default)] kit: Option<KitReference>` field
  to `ProfileDescriptor` in `src/profile/mod.rs`, render it in
  `forge profile inspect`, and confirm that every existing field, every existing
  profile id and the five v0.1 profile ids keep their current meaning.
- [x] 2.3 Render the profile-declared kit into the generated `forge.yaml`
  `kit` block (`id`, `version`, `ecosystem`, `tfm`, `confirmed_packages[]`,
  `provisional_packages[]`, `feed`, `zero_reason`, `exception?`) and add the
  additive pinned kit id and version to the project record with no new SQLite
  table and no new migration.
- [x] 2.4 Render the `aspnet-web` pre-wired manifest: the confirmed package set
  as `PackageReference` items, a `Directory.Packages.props` with
  `ManagePackageVersionsCentrally` and one `PackageVersion` per kit package, and
  every below-bar package inside a comment block naming its current external
  consumer count. Drive the TFM from the kit descriptor (pending §1.6), and
  confirm the confirmed set restores with no database, broker, cache or
  provider account.
- [x] 2.5 Implement the feed validator and renderer in `src/kit/feed.rs`: a
  named source with a value **relative to the generated project**, cleared
  inherited sources, and the neutral public source alongside it; refusal of an
  absolute path, a `..`-escaping path, an escaping `file://` URL, a
  source-mode `ProjectReference` or path dependency, a shell metacharacter, a
  secret-shaped value, and any value resolved from an environment variable —
  each with `kit-feed-invalid`, before staging, without echoing the value.
- [x] 2.6 Implement the floor in `src/kit/floor.rs`: met, unmet (typed
  `kit-floor-not-met` before any staging, with the confirmed and provisional
  sets named in the message), `--kit-exception <reason>` (visible, dated
  exception in `forge.yaml` and the README, plus a `WARN`), and
  `kit-exception-reason-required` for an empty reason. Confirm no warn-only
  default path exists anywhere in the code.
- [x] 2.7 Implement the declared zero as a distinct state: profiles whose
  ecosystem has no registered kit render normally, record
  `minimum_packages: 0` with a `zero_reason`, print the reason, and are never
  reported as having met their floor in either the manifest or the console
  output.
- [x] 2.8 Vendor the token assets for `react-web` and `nextjs-web` through
  `src/kit/assets.rs`: the owned subtree, the one-digest-per-file ownership
  receipt, and `kit-digest-mismatch` on drift. Confirm the project's build and
  test commands stay offline and that no registry dependency was added.
- [x] 2.9 Bind the presentation layer to exactly one token source: pattern
  adapters in `src/ui_pattern/mod.rs` consume the vendored tokens, Forge emits
  no second palette, spacing scale or typography definition, and `flutter-app`
  records a token absence with a reason instead of synthesizing Dart tokens or
  substituting copied web markup.
- [x] 2.10 Add the digest-agreement and registry-completeness tests: a drifted
  `kits/` byte fails naming the file, the expected and the actual digest; a
  profile declaring a kit with no registry row fails; a kit declaring an
  unclassified package fails. Both mirror the existing
  `platform-contract-consumption` discipline rather than introducing a new one.
- [x] 2.11 Implement the explicit upgrade path: a reviewable per-file diff
  against the new descriptor, refusal with the existing ownership-conflict code
  when an owned file was edited, and an explicit "upgrade unavailable, with a
  reason" state when an ecosystem has no upgrade path. Confirm no other code
  path — `forge new`, `forge doctor`, `forge list`, a CI run, a background
  process — can rewrite a pinned project.
- [x] 2.12 Add the six typed errors (`kit-unknown`, `kit-ecosystem-mismatch`,
  `kit-feed-invalid`, `kit-floor-not-met`, `kit-exception-reason-required`,
  `kit-digest-mismatch`) and their `code()` arms, and confirm each refusal
  leaves the destination, the staging area and the registry untouched.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-audit every surface the change touches: CLI (`forge new`,
  `forge profile inspect`), the generator, the registry, the project manifest,
  the existing `.standard/` standard-pack path and the owned-file conflict
  behaviour. Assert the new fields are additive and that `Cargo.toml`,
  `deny.toml` and the MCP tool registry, API route table and portal section set
  gain nothing.
- [x] 3.2 Re-audit the product boundary end to end. Generate each profile on a
  clean host, build and test it with its own toolchain and the Forge binary
  absent, and confirm the pinned kit reference does not create a Forge runtime
  dependency. Where a toolchain is missing, record the result as `unverified`
  and never as a pass.
- [x] 3.3 Prove determinism is intact: repeated `forge new` runs and the
  interactive path produce byte-identical output including the rendered kit
  manifest and the ownership receipt, and the README's "Rendering is verified
  against the asset version" claim still holds with the kit assets added.
- [x] 3.4 Prove the failure boundaries. Occupy the destination with a
  non-empty directory, tamper with a vendored asset, supply an absolute feed
  path, and request a profile whose floor is unmet — each refuses with its typed
  error, writes nothing, and leaves no half-wired project. Confirm a
  `dotnet-platform-libs` sibling checkout is never read and no network call
  occurs on any generation path.
- [x] 3.5 Prove no capability is granted by reference. Confirm a scaffold that
  references `Platform.Idempotency` or `Platform.RateLimiting` ships no
  idempotency, rate limiting, auth, database, billing, tenancy or deployment
  behaviour, and that infrastructure-bearing packages appear only behind an
  explicit feature request.
- [x] 3.6 Confirm the scope boundary. Run the existing suite and confirm that
  nothing about the 81 existing repositories, `forge gate` semantics, the
  adoption gate, publish, delivery, Studio or the project-to-production workflow
  changed. Any difference outside the scaffold surface is a scope violation for
  this package.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, focused `cargo test kit`, the
  generator render-fixture tests, `cargo clippy --all-targets` with no new
  findings, `git diff --check`, and
  `openspec validate --all --strict --no-interactive`. Record the actual
  output, not the intent.
- [x] 4.2 Run the native scaffold verification on a toolchain-equipped host
  for `aspnet-web` (restore plus build), `react-web` and `nextjs-web`
  (`npm run build` and `npm test` offline), `rust-web` and `flutter-app`. Record
  each result honestly; a toolchain that is absent is reported `unverified` and
  is not presented as a pass.
- [x] 4.3 Record the evidence honestly in `HANDOFF.md`: the per-package
  consumer counts the floor was derived from, the confirmed and provisional set
  boundaries, the resolved answers to the four open decisions, and any
  verification gap left open. Do not label a digest or render test as native
  restore evidence.
- [x] 4.4 Confirm the archive precondition: every scoped requirement and
  scenario in `specs/scaffold-prewires-shared-layer/spec.md` has an owning test,
  the `HANDOFF.md` `current_spec` pointer names this change, and
  `node scripts/check-openspec-change-names.mjs` passes. Archive only under
  explicit owner direction, never with `--skip-specs`.

## 5. Amendment: the committed repo-relative feed (owner ruling)

The owner rejected the environment-variable feed mechanism. The amendment below
replaces it with the workspace convention the two working consumers already use
(`therapist-commons`, `citylens`), makes the feed reproducible, and adds the
drift gate that the `global.json`-versus-CI defect showed was missing.

- [x] 5.1 **Read the proven convention before designing.** Confirm
  `therapist-commons/nuget.config` (`<clear />` + `local-platform` →
  `.packages/local-feed` + nuget.org, 21 committed `.nupkg`, plus
  `scripts/pack-platform.sh` to regenerate) and `citylens/nuget.config`
  (`artifacts/platform-feed`, 6 committed `.nupkg`) are the same shape, and
  match it rather than inventing a third. Read-only; neither repository was
  modified.
- [x] 5.2 Amend the **spec** first: rewrite "Named versioned feed" as
  "Named versioned repo-relative feed", add the requirement that no generated
  file resolves the feed through an environment variable or a restore-time
  source override, and add two requirements — *the feed contents are committed
  and reproducible*, and *the committed feed is verified against the declared
  kit version*.
- [x] 5.3 Amend `design.md` §1, §3, §5, §10 and §11 and the §12 decision
  ledger: record the environment-variable mechanism as **considered and
  rejected with its reason**, record the `forge kit` exception to "no new
  top-level CLI verb", and record that the restore closure (9) is larger than
  the confirmed set (6).
- [x] 5.4 Amend `proposal.md`: the What Changes ruling, the persistence and
  security surfaces, the failure list, the verification list, the capability
  statement, the narrowed `deterministic-project-generation` scope, and the
  non-goal that previously forbade packing the shared layer.
- [x] 5.5 Replace `FeedRef.env` with `FeedRef.path` in `src/kit/registry.rs`,
  `src/core/manifest.rs` and `src/profile/mod.rs`; delete
  `NUGET_FEED_ENV`, `FEED_ENV_VARS` and `feed_env`; render a `NuGet.config`
  with `<clear />`, the named project-relative source and nuget.org; remove the
  `RestoreAdditionalProjectSources` block from `Directory.Packages.props` and
  the `ARG`/`ENV` feed lines from the generated `Dockerfile`.
- [x] 5.6 Declare the **restore closure** in the descriptor — the six confirmed
  packages plus `Platform.Billing.Contracts`, `Platform.Eventing` and
  `Platform.Web.Telemetry` — and make the pack command and the drift check read
  it, so the floor still counts only the confirmed set.
- [x] 5.7 Vendor the feed: `forge kit pack` repacks the closure from a
  checked-out sibling, taking the version from the sibling's own
  `eng/package-manifest.json`, packing with an argument array, writing only
  inside this repository, pruning undeclared files, and recording one digest per
  packed file in `kits/manifest.json`. A missing sibling or toolchain is a
  refusal that changes nothing.
- [x] 5.8 Write the committed `.nupkg` bytes into the generated project. They
  are binary, so they are staged and promoted through the same path as the text
  files rather than through the text `template_files` vector; digest drift
  fails before anything is staged.
- [x] 5.9 Add `forge kit verify` and the four drift failures it must produce:
  a version mismatch against the declared `kit.version`, a missing package, an
  undeclared package, and a tampered or unrecorded byte. Run it against
  `forge.yaml` as well as the compiled-in descriptor, so the check reads what
  the project *says* it pins.
- [x] 5.10 Put the drift gate in `cargo test`, not only in a command, because a
  check only a human runs is a check CI does not run — which is how the
  `global.json` defect shipped.
- [x] 5.11 Fix the **composed provisional-reason defect** found in the
  already-implemented work: two `PLATFORM_REQUIRES_INFRA` reason strings
  embedded a consumer count the composer also emits, rendering
  "3 external consumers recorded, but 3 consumers, but needs…". Remove the
  embedded counts, make a doubled separator structurally impossible, and add a
  test over the composed strings so it cannot regress.
- [x] 5.12 Update the generated `README.md`, the manifest `kit.feed` block, the
  `forge profile inspect` line and the module doc comments, so no comment
  anywhere still justifies the rejected mechanism.
- [x] 5.13 **Prove portability end to end** — the oracle for the owner's
  requirement. Render an `aspnet-web` scaffold into one directory, copy it to an
  unrelated path, and run the project's own `dotnet restore` and `dotnet build`
  there with the feed variable removed and no sibling library present. Verbatim
  commands and output are in `HANDOFF.md`.

## 6. Notes on completion

Every checkbox is now checked. Points are recorded here so a later reader is
not misled by the tick marks.

**The feed is a repo-relative directory with committed bytes, and the restore
closure is 9 packages, not 6.** The confirmed set stays 6 and the floor still
counts only those 6. A restore additionally resolves `Platform.Billing.Contracts`,
`Platform.Eventing` and `Platform.Web.Telemetry`, so the feed carries those
three as *transitive* members — present because the confirmed set needs them,
never as a direct reference and never counted toward the floor.

**`forge kit` is a deliberate, recorded exception to "no new top-level CLI
verb".** The feed is committed, so it needs a reproducible way to regenerate
(`pack`) and a check that catches drift (`verify`). Both are `cargo test`-covered;
a `scripts/pack-platform.sh` beside `kits/` would be neither. `design.md` §1
records the exception and its reason.

**2.11 has no CLI verb, on purpose.** The reviewable per-file diff, the
ownership-conflict refusal and the explicit "upgrade unavailable, with a
reason" state are implemented and tested as `kit::diff_kit_snapshot` and
`kit::upgrade_kit_snapshot`. They are not exposed under a new top-level verb,
because `design.md` section 1 fixes the extension points and states "**No new
MCP tool, API route, portal section or top-level CLI verb**". Wiring a verb is a
one-line follow-up once the owner wants one.

**4.2 and 5.13 native evidence is real, and the feed is committed.**
`aspnet-web` restores and builds clean on `net10.0` (0 warnings, 0 errors) with
Forge absent, at a path it was not generated at, with the feed variable removed
and no sibling library present — because the `.nupkg` bytes are committed inside
the project. This supersedes the earlier note in this file that recorded the
build as depending on `NUGET_PLATFORM_FEED`.

**Two recorded deviations from the design's wording.**

1. "**No new migration**" (section 8) is honoured in intent, not literally. The
   pinned kit id and version are additive observed columns on the existing
   `projects` row, applied through the repository's own established additive
   `ALTER TABLE` mechanism. No new table exists and no existing field changed
   meaning, but a new column does need that mechanism.
2. `design.md` section 4 claims the confirmed set's "only project references
   are `Platform.Core` and `Platform.Web.Telemetry`". Measured, `Platform.Testing`
   also references `Platform.Billing.Contracts` and `Platform.Eventing`. The
   conclusion still holds - every external reference in the whole closure is a
   `Microsoft.Extensions.*` abstraction, so the restore needs no database,
   broker, cache or provider account - but the sentence was not literally
   accurate.
