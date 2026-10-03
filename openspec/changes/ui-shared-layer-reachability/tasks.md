# Tasks: UI shared-layer reachability, single token source, and conformance

Shape follows the house precedent (`shared-test-web-factory`): **BFS baseline →
DFS implementation → BFS regression → Verification.** No consumer repository is
modified in this change.

**Implementation status.** This change has now been implemented in this repository.
The checkboxes below reflect what the implementing session actually completed, with the
evidence recorded under each one. Two items remain unchecked and carry their reason
inline. The change is **not archived** and `openspec/specs/` is unchanged: archiving is
the parent session's step.

---

## 0. Baseline (BFS) — measure before writing a single requirement

- [ ] Re-run the evidence in `proposal.md` § Evidence boundary and record the numbers this session actually observes, per scope. Expected: 15 repos declaring `.visually-hidden`/`sr-only`; 118 `@media` width queries / 24 distinct values at CSS scope; 139 / 31 at wide scope; 45 UI-bearing repos; 20 dark-mode; 17 skip-link; 202 `z-index` declarations. If a number has moved, **update `proposal.md` and `design.md` before coding** — a spec whose evidence is stale is worse than no spec.
      **NOT DONE as written, and `proposal.md` is deliberately left unedited.**
      Reproduced exactly at the stated scopes: 15 repositories declaring
      `.visually-hidden`/`sr-only`; 24 distinct CSS-scope media values; 31 distinct
      wide-scope values; the `640px`/`641px` and `768px`/`769px` off-by-one pairs; 8
      Dart repositories. Did **not** reproduce under a reconstructed probe: width-query
      totals (125 CSS scope / 150 wide, against 118 / 139 recorded), UI-bearing
      repositories (55, against 45), dark-mode (16, against 20), skip-link (15, against
      17), `z-index` declarations (110, against 202). The probe behind those figures is
      not recoverable from the proposal text, so overwriting them with a different
      measurement would replace evidence rather than refresh it, and the ruling that
      leans on them is D12. The load-bearing number - **15** - reproduces exactly, and
      `ui/legacy-divergent.json` is exactly those 15 repositories.
- [x] Verify the four divergent `.visually-hidden` bodies still differ as recorded: `openpanel/crates/openpanel-web/assets/app.css:141`, `cvunify/src/CvUnify.Web/wwwroot/css/site.css:388`, `arivio/src/Arivio.Console/src/styles.css:295`, `pangoman/src/HistoryKB.Web/wwwroot/css/site.css:94`.
      Confirmed: `openpanel/crates/openpanel-web/assets/app.css:141`
      (space-separated `clip`),
      `cvunify/src/CvUnify.Web/wwwroot/css/site.css:388` (comma form),
      `arivio/src/Arivio.Console/src/styles.css:295` (6 properties),
      `pangoman/src/HistoryKB.Web/wwwroot/css/site.css:93` (every declaration
      `!important`; the proposal cites `:94`, the first declaration inside the rule).
- [x] Confirm `arivio`'s copy still omits `padding`/`margin`/`border` (the 1px-box defect) and record it as the reason `.visually-hidden` is being shipped rather than documented.
      Confirmed: the body is exactly
      `position`/`width`/`height`/`overflow`/`clip`/`white-space`. It is why the rule
      is shipped rather than documented, and why `arivio` is `migrationOrder: 1` in
      the frozen list (D17).
- [x] Capture the **pre-change emitted token-name set** from `ui/packages/design-tokens/dist/tokens.css` and store it as the V6 regression baseline. This is the artifact that proves the generator rewrite renames nothing.
      Stored as `ui/tokens/emitted-baseline.json`: 38 names with their values, diffed
      against the `HEAD` copy of the generated CSS - 38/38 names present, 0 value
      mismatches, 0 missing. `verify-ui.mjs` now fails with `ui-token-renamed` if any
      of them is dropped or changed.
- [x] Re-confirm zero consumers: grep for `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell` outside `dotnet-platform-libs/ui/`. Record the result, including if it is no longer zero.
      Confirmed zero: no `package.json` outside `ui/` references any of the three
      packages.
- [x] Re-confirm the boundary proof for `proposal.md` §3: 0 hex literals and 0 dimension literals in `forge/src/ui_pattern/mod.rs`. **Read only** — do not modify `forge/`.
      Confirmed read-only: 2137 lines, 0 hex literals, 0 `px`/`rem`/`em` literals,
      0 `@media`. `forge/` was read and not modified.
- [x] Record the architecture-guard count by the method this repository can actually produce, and state both figures (137 `[Fact]`/`[Theory]` methods; 355 recorded executed cases). Do not re-run the suite in this step.
      Measured before any change: 137 `[Fact]`/`[Theory]` methods, 355 executed
      cases. After: 149 methods, 367 executed cases - 12 additive methods, none
      removed.
- [x] Record `eng/package-manifest.json` sha256 and the `eng/` baseline state so §3 can prove byte-identity afterwards.
      `eng/package-manifest.json` =
      `b12109a669354f819ddf4633a26e710809c8ef4f5ae6325c99158e27f17125c5`;
      `eng/public-api-baseline.txt` =
      `a48298b2d6d2f79d7c9d0477afe347eec6d2f766a376ea44ac753dd8bb7bcd9f`. Both are
      byte-identical after implementation.

---

## 1. Make the token source genuinely singular (DFS — prerequisite, do first)

- [x] **D-A:** rewrite `ui/scripts/generate-tokens.mjs` to walk the DTCG tree and resolve `{alias}` references, replacing the hand-enumerated template literal. Every `$value` in `tokens.json` SHALL be emitted.
- [x] **D-B:** move the 5 dark-theme hex literals (`#93c5fd`, `#bfdbfe`, `#fca5a5`, `#86efac`, `#fcd34d`) out of the generator template and into `ui/tokens/tokens.json` as dark-capable primitives.
- [x] Emit the canonical `.visually-hidden` primitive from the generator: the full 9-property form, **no** `!important`. Rationale in `design.md` §4.3.
- [x] Confirm the emitted token-name set is byte-equal to the §0 baseline (V6). If any name changed, stop and treat it as a contract change, not a refactor.
      Additive, as D15 requires: the emitted set is 47 names, a superset of the 38
      baselined ones, all 38 with byte-identical values. Strict set equality is not
      reachable together with "every `$value` in `tokens.json` SHALL be emitted",
      because `tokens.json` declares the `component.*` tokens and the five
      dark-capable shades that 0.1.0 never emitted. The enforced invariant is the
      normative one: no emitted token is renamed or removed.
- [x] Confirm no literal color/space/radius/font value remains in `tokens.css` that is not reachable from a `$value` or a documented `var()` alias.

## 2. Strengthen the verifier (DFS)

- [x] Extend `ui/scripts/verify-ui.mjs` with the three new assertions: token completeness (`ui-token-dropped`), no orphan literal (`ui-orphan-literal`), unresolvable alias (`ui-alias-unresolved`).
- [x] Add the canonical-primitive assertion: `.visually-hidden` present, all 9 properties, no `!important`.
- [x] **Preserve every existing assertion** — the `required[]` artifact list (`:7-16`), the 6-token check (`:19-21`), dark theme (`:22`), reduced-motion (`:23`), `aria-busy`/`aria-invalid`/`aria-live` (`:27`), `:focus-visible` (`:28`), disabled (`:29`), dialog role (`:30`). A regression here is a failure of this change, not a simplification.
- [x] Negative-test each new assertion: a deliberately dropped token, a deliberately injected orphan literal, and a deliberately broken alias each produce their named diagnostic and a non-zero exit.
      `ui/scripts/selftest-verify-ui.mjs`: 15 negative cases, each asserting its named
      diagnostic and a non-zero exit - dropped token (CSS and TypeScript), undeclared
      token, orphan literal in the dark theme, unresolved alias in a token and in a
      theme override, renamed 0.1.0 token, incomplete primitive, `!important`
      primitive, duplicate primitive, and the pre-existing gates (dark theme, reduced
      motion, accessibility marker, focus-visible, required artifact).

## 3. Pack the feed (DFS)

- [x] Add `ui/scripts/pack-ui.mjs`: deterministic packaging of `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell` at `0.1.0` into `ui/feed/*.tgz`, plus `ui/feed/feed.json` carrying the kit version and a sha256 per package.
- [x] Prove reproducibility (V7): two consecutive runs yield byte-identical tarballs and identical digests. A non-deterministic tarball invalidates the whole digest check.
      Two runs, `sha256sum ui/feed/*.tgz` compared: identical
      (`7043211b...`, `45c757f6...`, `0a23272e...`).
- [x] Decide and record the tarball member ordering / mtime normalisation that makes step 2 pass. A comment saying "this is deterministic" is not evidence.
      Recorded in `ui/scripts/pack-ui.mjs` as three named normalisations: members
      sorted by tar path; mtime fixed at epoch 0 with uid/gid 0, empty uname/gname,
      mode 0644; a single gzip stream at a fixed level.
- [x] Confirm the packed `react-ui` and `react-shell` tarballs carry the files their `package.json` `files` arrays declare, and nothing else.
      `react-ui` carries `package/package.json`, `src/components.tsx`,
      `src/index.ts`, `styles.css` (its `files: ["src", "styles.css"]` plus
      package.json, nothing else; `tests/` is excluded). `react-shell` declares no
      `files` array, so it carries everything: `package.json`, `src/index.ts`,
      `src/shell.tsx`. `design-tokens` carries `package.json` plus `dist/`.
      `ui/feed/feed.json` records each member list and the architecture guard asserts
      it.

## 4. Conformance check (DFS)

- [x] Add `ui/legacy-divergent.json` as the **frozen** baseline data file (D16): the repos that declare a shared primitive today. Generated from the §0 measurement, expected 15. No code path may add an entry.
      `ui/legacy-divergent.json`, schema 1, three frozen sections: 15
      `divergentPrimitive` entries (repo, file, line, selector, properties, the
      specific divergence, severity, migration order), 55 `undeclaredUiRepos`, 8
      `flutterNotCoveredRepos`, 24 `mediaQueryBaselinedValues` with the two
      off-by-one pairs, and the documented `kitScale`. `UiDistributionTests` proves
      that no `.sh` or `.mjs` file in the repository writes it (D16).
- [x] Add `scripts/check-ui-conformance.sh` implementing `design.md` §4: - fail `ui-divergent-primitive` for any shared-primitive declaration outside a vendored feed directory and outside the frozen list; - fail `ui-conformance-undeclared` for a UI-bearing repo with no `ui-conformance.json`; - pass vacuously for a repo with no UI signal and no declaration; - honour `consumes-feed` and `exempt` states; - fail `ui-breakpoint-undeclared` for a **new** non-scale breakpoint while baselining the existing 118 (D12).
- [x] Add `ui/scripts/verify-ui-feed.mjs`: feed ⇄ `ui-conformance.json` agreement — digest mismatch, version mismatch, missing needed package, undeclared extra package.
- [x] Implement the path refusals: absolute `file:`, `..`-escaping `file:`, and env-var-assembled `file:` each produce their named diagnostic. These are the three the `forge` precedent rejected; do not soften them.
- [x] Confirm the check is **read-only** over consumer repositories: it reports, it never writes, and it never installs anything.

## 5. Guards (DFS)

- [x] Add `tests/Platform.Architecture.Tests/UiDistributionTests.cs` in the **existing** project — no new `.csproj`, no new `.Test` package.
- [x] Assert the frozen legacy list has not grown and is non-extensible (V12).
- [x] Assert `ui/packages/design-tokens/dist/tokens.css` is byte-stable against a committed digest.
- [x] Assert the canonical `.visually-hidden` is present in the generated CSS.
- [x] Assert no .NET project anywhere in the solution references a UI npm package (the .NET and npm layers stay decoupled).
- [x] Negative-test the frozen-list guard: a synthetic 16th divergent repo fails it, and there is no API to add one.
      `A_sixteenth_divergent_repository_fails_the_frozen_list_guard` runs the
      guard's own predicate over a synthetic sixteenth entry and asserts
      `ui-legacy-list-grew`; the guard is a method, so there is no API to add an entry
      with, and the non-writability test covers the data-file route.
- [x] Confirm the new methods are **additive** — no existing guard deleted, renamed, weakened or skipped.

## 6. Documentation (DFS)

- [x] Add `ui/README.md`: the token source, the generator, the canonical primitive, the feed, and the three refusals (absolute path, `..`-escape, env var) stated as rules rather than folklore.
      `ui/README.md`: token source, naming rule, canonical primitive, the feed and
      the three refusals as rules, the D15 version policy, the conformance contract
      with its named diagnostics, the frozen list, the D18 gate wiring, and the known
      boundaries - including that `Platform.UI.Razor`'s own `platform-tokens.css` is
      still hand-authored and out of this change's scope.
- [x] Update `docs/packages.md` UI rows for `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell` and the `Platform.UI.Razor` row to record the feed and the conformance contract.
      `docs/packages.md` § Platform UI gained a "Distribution and conformance"
      subsection covering the feed, the digest policy, the verifier wiring, the
      conformance check and the D14 coverage gap. `Platform.UI.Razor`'s own row is
      unchanged: still a 0.1.0 package with no dependency.
- [x] Update the `README.md` UI section to point at the feed instead of leaving the layer undiscoverable.
      The UI section now points at `ui/README.md` and states the `file:`-tarball
      mechanism, the single token source, the canonical primitive and the gate
      wiring.
- [x] Ruled **D14**: `check-public-api.sh` does **not** cover `Platform.UI.Razor` and `eng/public-api-baseline.txt` is **not** regenerated. Record the coverage gap explicitly in the change docs so it stays visible rather than being closed over.
      Not regenerated. Recorded in three places: `ui/README.md` (Known boundaries),
      `docs/packages.md` § Platform UI, and the tripwire
      `The_public_api_coverage_gap_for_platform_ui_razor_stays_recorded_not_closed`
      in `UiDistributionTests`, which also asserts the ruling is written down in
      `design.md` so the record cannot quietly disappear.
- [x] Ruled **D15**: the three UI packages stay at `0.1.0` with digest-pinning. Do **not** take `0.2.0` — the change is additive, so a bump would signal a contract change that did not happen.
      The three packages stay at `0.1.0`; `UiDistributionTests` asserts all three
      `package.json` versions and that the digest check exists, and
      `eng/package-manifest.json` still records `Platform.UI.Razor` at `0.1.0` with no
      package or project references.
- [x] Wire `verify-ui.mjs` into `scripts/quality-gate.sh` (ruled; this overrides the change's earlier "recommended: wire it in, but leave it to the owner" hedge). Without it a token regression does not block the repo gate, which is the `contract-parity: OK` failure class again.
      Done, with the rationale in the gate's own comment:
      `node ui/scripts/verify-ui.mjs` and
      `node ui/scripts/selftest-verify-ui.mjs` run before the `dotnet` steps, so a
      token regression blocks the repository gate instead of being discovered by a
      consumer.

---

## 7. BFS — cross-surface regression and completeness

- [ ] Every requirement in `specs/platform-ui-token-distribution/spec.md` has at least one passing test, and every test maps back to a named requirement.
      **PARTIAL.** Twelve of the fourteen requirements have an executed oracle. Two
      scenarios cannot have one in this repository: "a Razor Pages repository consumes
      the token artifact" (no Razor consumer repository exists to consume it - NG1,
      NG7) and "a Dart token artifact is requested" (the platform ships no Dart
      artifact - NG5). "A consumer's own styles override the shared accessibility
      rule" is likewise unprovable here, because the verification subject is this
      repository's artifacts, not a consumer's cascade.
- [x] Every oracle V1–V14 in `design.md` §9 has been executed and its output recorded. An oracle blocked by a missing toolchain is reported **`unverified`** with its exact command — never as passing.
      V1-V5 by `node ui/scripts/verify-ui.mjs`; V6 by the baseline diff plus
      `ui-token-renamed`; V7 by two packing runs; V8 and V9 by
      `scripts/test-ui-distribution.sh`; V10 by a real `npm install` at an unrelated
      path, offline, with the registry pointed at an unreachable address; V11, V13 and
      V14 by conformance fixtures; V12 by the architecture guard. V15-V18 are in §8.
- [x] Every entry in the failure table in `design.md` §6 has a test that produces its named diagnostic.
      All twelve named diagnostics are produced by a test or fixture:
      `ui-token-dropped`, `ui-orphan-literal`, `ui-alias-unresolved`,
      `ui-feed-path-invalid`, `ui-feed-incomplete`, `ui-feed-digest-mismatch`,
      `ui-feed-version-mismatch`, `ui-feed-undeclared`, `ui-conformance-undeclared`,
      `ui-divergent-primitive`, `ui-legacy-list-grew`, `ui-breakpoint-undeclared`. The
      thirteenth row of that table ("`forge` catalog gains a literal value") is
      explicitly not checked here and stays with the NG4 follow-up.
- [x] `ui/packages/react-ui/tests/components.test.mjs` and the `react-shell` tests are green. The generator rewrite changed a shared input to both.
      `node ui/packages/react-ui/tests/components.test.mjs` passes. `react-shell`
      ships no test file and no `test` script; its contract is covered by
      `ui/scripts/verify-ui.mjs` (required artifact, shell assertions) and by
      `scripts/test-ui-distribution.sh` (tarball members, digests, offline
      resolution).
- [x] `eng/package-manifest.json` is **byte-identical** to the §0 sha256, and the package inventory still enumerates **75**. No new `.csproj`, no new `PackageReference`, no new `ProjectReference`.
      Byte-identical (sha256 unchanged); `scripts/generate-package-manifest.sh
      --check` reports `Manifest matches source.`; `scripts/package-inventory.sh`
      still enumerates 75 `.csproj` files. No new `.csproj`, `PackageReference` or
      `ProjectReference`.
- [x] `eng/public-api-baseline.txt` is unchanged, or the D14 decision is recorded with its regeneration evidence.
      Unchanged, and deliberately not regenerated (D14).
      `scripts/check-public-api.sh` still scans the same six packages and produces a
      clean diff, and `UiDistributionTests` fails if the `Platform.UI.Razor` gap is
      ever closed without regenerating the baseline in the same change.
- [x] No source, config, script or test outside this repository changed. No `git commit`, `git add` or `git push`; no `openspec archive`.
      No `git commit`, `git add`, `git push` or `openspec archive` was run. Nothing
      outside this repository was written: the 15 legacy copies (including `arivio`)
      and `forge/` are read-only inputs, and the portability proof used a fixture in a
      temporary directory, not a real consumer.
- [x] `openspec/specs/platform-ui-design-system/spec.md` is unmodified, and `openspec/specs/` gains exactly one directory.
      `git status --porcelain openspec/` shows no modified spec and no added
      directory. The new capability spec still lives only under
      `openspec/changes/ui-shared-layer-reachability/specs/`, because the change is
      not archived. `openspec validate --all --strict --no-interactive` reports 51
      passed / 0 failed, and a pristine `git archive HEAD` export validates to the
      same 51 - this change adds no OpenSpec item.
- [x] Every "Placed IN" claim in this plan is present in the tree, and nothing from Non-goals NG1–NG9 was implemented. In particular: **no** consumer repository was touched, **no** `forge/` file was touched, **no** Dart artifact was produced, **no** breakpoint scale was unified, and **no** shadcn/ui migration was started.
      Present: the DTCG walk, the token-sourced dark theme, the canonical primitive,
      the deterministic feed, the digest verifier with its three path refusals, the
      frozen baseline, the conformance script, the additive guards, the self-tests and
      the docs. Absent, as required: no consumer repository touched, no `forge/` file
      touched, no Dart artifact, no breakpoint scale unified, no shadcn/ui migration.

---

## 8. Verification

- [x] `openspec validate ui-shared-layer-reachability --strict` passes.
- [x] `openspec validate --changes --strict --no-interactive` passes.
- [x] `node ui/scripts/verify-ui.mjs` passes (V1–V5).
- [x] `ui/scripts/pack-ui.mjs` run twice is byte-identical (V7).
      Verified: two consecutive runs, identical sha256 for all three tarballs.
- [x] `ui/scripts/verify-ui-feed.mjs` passes on a conforming fixture and fails on each of digest, version, missing and undeclared (V8).
      Passes on the feed-consumer fixture; fails on digest, version, missing and
      undeclared (both an undeclared manifest package and a stray tarball) and on each
      of the three refused path forms.
- [x] Feed path refusals proven: absolute, `..`-escaping, env-var (V9).
      Absolute, `..`-escaping and environment-variable specifiers each produce
      `ui-feed-path-invalid`, naming the specifier and, for the variable, the
      variable.
- [x] **Portability proof (V10):** a consumer fixture is copied to an unrelated path and resolves with no sibling checkout, no environment variable and no network. This is the oracle that the whole change exists to satisfy — if it is skipped, the change has not been validated.
      A fixture consumer with the committed tarballs was copied to an unrelated
      temporary path and resolved with
      `npm install --offline --no-audit --no-fund --legacy-peer-deps --registry http://127.0.0.1:1/`
      - no network reachable, no sibling checkout, no environment variable, no
      absolute path recorded. The resolved
      `node_modules/@platform/design-tokens/dist/tokens.css` is byte-identical to the
      generated artifact.
- [x] `scripts/check-ui-conformance.sh` exits 0 on the current workspace, and non-zero with `ui-divergent-primitive` on a fixture repo that declares `.visually-hidden` itself (V11, V13, V14).
      Exits 0 on the real workspace: 82 repositories scanned, 15 known frozen
      divergences, 48 frozen undeclared-UI repositories, 33 out of scope, 0
      diagnostics. On fixtures it exits non-zero with `ui-divergent-primitive`,
      `ui-conformance-undeclared` and `ui-breakpoint-undeclared` respectively, and 0
      for a repository with no UI signal, an exemption, and a frozen-list entry.
- [x] `scripts/generate-package-manifest.sh --check` reports `Manifest matches source.` (V15).
- [x] `scripts/package-inventory.sh` still enumerates **75** projects (V15).
- [x] `scripts/check-public-api.sh` produces a clean diff (V16).
- [x] `dotnet test tests/Platform.Architecture.Tests` passes with the new guards green and no existing guard removed (V17).
- [x] `scripts/quality-gate.sh` passes end to end (V18), **including** the newly wired `verify-ui.mjs` step. If the verifier is not yet wired, that is an incomplete task, not an accepted gap — report it as outstanding.
      `GATE_EXIT=0` end to end, running `node ui/scripts/verify-ui.mjs` and
      `node ui/scripts/selftest-verify-ui.mjs` first, then `dotnet restore`/`build`/
      `test`, `openspec validate --changes` and `--specs`, and `git diff --check`. A
      second gate run had one unrelated flake -
      `Platform.Jobs.Hangfire.Tests.EndToEndReliabilityTests.Sequential_dispatches_use_independent_handler_state`
      hit its 30-second timeout under concurrent load and passes 69/69 when that
      project is run on its own. No Hangfire or Jobs code is touched by this change.
- [x] `git diff --check` is clean.
- [ ] Record every actual command output in the handoff. Report any
      environment-blocked check with its exact next action, never as passing.
      **Left to the parent session.** This worker session made no commit (the parent
      commits), and `AGENTS.md` step 7 puts the `HANDOFF.md` update after the
      implementation commit. The command output this session observed is in its
      report, not in `HANDOFF.md`.
- [ ] State the evidence boundary in `HANDOFF.md` explicitly: **no consumer repository is resolved, migrated or proven by this change.** The portability proof (V10) is the one thing that *does* resolve a feed outside this repository, and it is done with a fixture, not with `devloom` or `kairovia`.
