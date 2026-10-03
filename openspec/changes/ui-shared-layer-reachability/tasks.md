# Tasks: UI shared-layer reachability, single token source, and conformance

Shape follows the house precedent (`shared-test-web-factory`): **BFS baseline →
DFS implementation → BFS regression → Verification.** No consumer repository is
modified in this change.

**All boxes are UNCHECKED. This change is a proposal, not an implementation.**

---

## 0. Baseline (BFS) — measure before writing a single requirement

- [ ] Re-run the evidence in `proposal.md` § Evidence boundary and record the
      numbers this session actually observes, per scope. Expected: 15 repos
      declaring `.visually-hidden`/`sr-only`; 118 `@media` width queries / 24
      distinct values at CSS scope; 139 / 31 at wide scope; 45 UI-bearing
      repos; 20 dark-mode; 17 skip-link; 202 `z-index` declarations. If a
      number has moved, **update `proposal.md` and `design.md` before coding** —
      a spec whose evidence is stale is worse than no spec.
- [ ] Verify the four divergent `.visually-hidden` bodies still differ as
      recorded: `openpanel/crates/openpanel-web/assets/app.css:141`,
      `cvunify/src/CvUnify.Web/wwwroot/css/site.css:388`,
      `arivio/src/Arivio.Console/src/styles.css:295`,
      `pangoman/src/HistoryKB.Web/wwwroot/css/site.css:94`.
- [ ] Confirm `arivio`'s copy still omits `padding`/`margin`/`border` (the
      1px-box defect) and record it as the reason `.visually-hidden` is being
      shipped rather than documented.
- [ ] Capture the **pre-change emitted token-name set** from
      `ui/packages/design-tokens/dist/tokens.css` and store it as the V6
      regression baseline. This is the artifact that proves the generator
      rewrite renames nothing.
- [ ] Re-confirm zero consumers: grep for `@platform/design-tokens`,
      `@platform/react-ui`, `@platform/react-shell` outside
      `dotnet-platform-libs/ui/`. Record the result, including if it is no
      longer zero.
- [ ] Re-confirm the boundary proof for `proposal.md` §3: 0 hex literals and
      0 dimension literals in `forge/src/ui_pattern/mod.rs`. **Read only** —
      do not modify `forge/`.
- [ ] Record the architecture-guard count by the method this repository can
      actually produce, and state both figures (137 `[Fact]`/`[Theory]`
      methods; 355 recorded executed cases). Do not re-run the suite in this
      step.
- [ ] Record `eng/package-manifest.json` sha256 and the `eng/` baseline state
      so §3 can prove byte-identity afterwards.

---

## 1. Make the token source genuinely singular (DFS — prerequisite, do first)

- [ ] **D-A:** rewrite `ui/scripts/generate-tokens.mjs` to walk the DTCG tree
      and resolve `{alias}` references, replacing the hand-enumerated template
      literal. Every `$value` in `tokens.json` SHALL be emitted.
- [ ] **D-B:** move the 5 dark-theme hex literals (`#93c5fd`, `#bfdbfe`,
      `#fca5a5`, `#86efac`, `#fcd34d`) out of the generator template and into
      `ui/tokens/tokens.json` as dark-capable primitives.
- [ ] Emit the canonical `.visually-hidden` primitive from the generator: the
      full 9-property form, **no** `!important`. Rationale in `design.md` §4.3.
- [ ] Confirm the emitted token-name set is byte-equal to the §0 baseline
      (V6). If any name changed, stop and treat it as a contract change, not a
      refactor.
- [ ] Confirm no literal color/space/radius/font value remains in
      `tokens.css` that is not reachable from a `$value` or a documented
      `var()` alias.

## 2. Strengthen the verifier (DFS)

- [ ] Extend `ui/scripts/verify-ui.mjs` with the three new assertions:
      token completeness (`ui-token-dropped`), no orphan literal
      (`ui-orphan-literal`), unresolvable alias (`ui-alias-unresolved`).
- [ ] Add the canonical-primitive assertion: `.visually-hidden` present, all 9
      properties, no `!important`.
- [ ] **Preserve every existing assertion** — the `required[]` artifact list
      (`:7-16`), the 6-token check (`:19-21`), dark theme (`:22`),
      reduced-motion (`:23`), `aria-busy`/`aria-invalid`/`aria-live` (`:27`),
      `:focus-visible` (`:28`), disabled (`:29`), dialog role (`:30`). A
      regression here is a failure of this change, not a simplification.
- [ ] Negative-test each new assertion: a deliberately dropped token, a
      deliberately injected orphan literal, and a deliberately broken alias each
      produce their named diagnostic and a non-zero exit.

## 3. Pack the feed (DFS)

- [ ] Add `ui/scripts/pack-ui.mjs`: deterministic packaging of
      `@platform/design-tokens`, `@platform/react-ui`, `@platform/react-shell`
      at `0.1.0` into `ui/feed/*.tgz`, plus `ui/feed/feed.json` carrying the
      kit version and a sha256 per package.
- [ ] Prove reproducibility (V7): two consecutive runs yield byte-identical
      tarballs and identical digests. A non-deterministic tarball invalidates
      the whole digest check.
- [ ] Decide and record the tarball member ordering / mtime normalisation
      that makes step 2 pass. A comment saying "this is deterministic" is not
      evidence.
- [ ] Confirm the packed `react-ui` and `react-shell` tarballs carry the files
      their `package.json` `files` arrays declare, and nothing else.

## 4. Conformance check (DFS)

- [ ] Add `ui/legacy-divergent.json` as the **frozen** baseline data file
      (D16): the repos that declare a shared primitive today. Generated from
      the §0 measurement, expected 15. No code path may add an entry.
- [ ] Add `scripts/check-ui-conformance.sh` implementing `design.md` §4:
      - fail `ui-divergent-primitive` for any shared-primitive declaration
        outside a vendored feed directory and outside the frozen list;
      - fail `ui-conformance-undeclared` for a UI-bearing repo with no
        `ui-conformance.json`;
      - pass vacuously for a repo with no UI signal and no declaration;
      - honour `consumes-feed` and `exempt` states;
      - fail `ui-breakpoint-undeclared` for a **new** non-scale breakpoint
        while baselining the existing 118 (D12).
- [ ] Add `ui/scripts/verify-ui-feed.mjs`: feed ⇄ `ui-conformance.json`
      agreement — digest mismatch, version mismatch, missing needed package,
      undeclared extra package.
- [ ] Implement the path refusals: absolute `file:`, `..`-escaping `file:`,
      and env-var-assembled `file:` each produce their named diagnostic.
      These are the three the `forge` precedent rejected; do not soften them.
- [ ] Confirm the check is **read-only** over consumer repositories: it
      reports, it never writes, and it never installs anything.

## 5. Guards (DFS)

- [ ] Add `tests/Platform.Architecture.Tests/UiDistributionTests.cs` in the
      **existing** project — no new `.csproj`, no new `.Test` package.
- [ ] Assert the frozen legacy list has not grown and is non-extensible (V12).
- [ ] Assert `ui/packages/design-tokens/dist/tokens.css` is byte-stable against
      a committed digest.
- [ ] Assert the canonical `.visually-hidden` is present in the generated CSS.
- [ ] Assert no .NET project anywhere in the solution references a UI npm
      package (the .NET and npm layers stay decoupled).
- [ ] Negative-test the frozen-list guard: a synthetic 16th divergent repo
      fails it, and there is no API to add one.
- [ ] Confirm the new methods are **additive** — no existing guard deleted,
      renamed, weakened or skipped.

## 6. Documentation (DFS)

- [ ] Add `ui/README.md`: the token source, the generator, the canonical
      primitive, the feed, and the three refusals (absolute path, `..`-escape,
      env var) stated as rules rather than folklore.
- [ ] Update `docs/packages.md` UI rows for `@platform/design-tokens`,
      `@platform/react-ui`, `@platform/react-shell` and the `Platform.UI.Razor`
      row to record the feed and the conformance contract.
- [ ] Update the `README.md` UI section to point at the feed instead of leaving
      the layer undiscoverable.
- [ ] Ruled **D14**: `check-public-api.sh` does **not** cover
      `Platform.UI.Razor` and `eng/public-api-baseline.txt` is **not**
      regenerated. Record the coverage gap explicitly in the change docs so
      it stays visible rather than being closed over.
- [ ] Ruled **D15**: the three UI packages stay at `0.1.0` with
      digest-pinning. Do **not** take `0.2.0` — the change is additive, so a
      bump would signal a contract change that did not happen.
- [ ] Wire `verify-ui.mjs` into `scripts/quality-gate.sh` (ruled; this
      overrides the change's earlier "recommended: wire it in, but leave it
      to the owner" hedge). Without it a token regression does not block the
      repo gate, which is the `contract-parity: OK` failure class again.

---

## 7. BFS — cross-surface regression and completeness

- [ ] Every requirement in `specs/platform-ui-token-distribution/spec.md` has
      at least one passing test, and every test maps back to a named
      requirement.
- [ ] Every oracle V1–V14 in `design.md` §9 has been executed and its output
      recorded. An oracle blocked by a missing toolchain is reported
      **`unverified`** with its exact command — never as passing.
- [ ] Every entry in the failure table in `design.md` §6 has a test that
      produces its named diagnostic.
- [ ] `ui/packages/react-ui/tests/components.test.mjs` and the `react-shell`
      tests are green. The generator rewrite changed a shared input to both.
- [ ] `eng/package-manifest.json` is **byte-identical** to the §0 sha256, and
      the package inventory still enumerates **75**. No new `.csproj`, no new
      `PackageReference`, no new `ProjectReference`.
- [ ] `eng/public-api-baseline.txt` is unchanged, or the D14 decision is
      recorded with its regeneration evidence.
- [ ] No source, config, script or test outside this repository changed. No
      `git commit`, `git add` or `git push`; no `openspec archive`.
- [ ] `openspec/specs/platform-ui-design-system/spec.md` is unmodified, and
      `openspec/specs/` gains exactly one directory.
- [ ] Every "Placed IN" claim in this plan is present in the tree, and nothing
      from Non-goals NG1–NG9 was implemented. In particular: **no** consumer
      repository was touched, **no** `forge/` file was touched, **no** Dart
      artifact was produced, **no** breakpoint scale was unified, and **no**
      shadcn/ui migration was started.

---

## 8. Verification

- [ ] `openspec validate ui-shared-layer-reachability --strict` passes.
- [ ] `openspec validate --changes --strict --no-interactive` passes.
- [ ] `node ui/scripts/verify-ui.mjs` passes (V1–V5).
- [ ] `ui/scripts/pack-ui.mjs` run twice is byte-identical (V7).
- [ ] `ui/scripts/verify-ui-feed.mjs` passes on a conforming fixture and fails
      on each of digest, version, missing and undeclared (V8).
- [ ] Feed path refusals proven: absolute, `..`-escaping, env-var (V9).
- [ ] **Portability proof (V10):** a consumer fixture is copied to an unrelated
      path and resolves with no sibling checkout, no environment variable and
      no network. This is the oracle that the whole change exists to satisfy —
      if it is skipped, the change has not been validated.
- [ ] `scripts/check-ui-conformance.sh` exits 0 on the current workspace, and
      non-zero with `ui-divergent-primitive` on a fixture repo that declares
      `.visually-hidden` itself (V11, V13, V14).
- [ ] `scripts/generate-package-manifest.sh --check` reports
      `Manifest matches source.` (V15).
- [ ] `scripts/package-inventory.sh` still enumerates **75** projects (V15).
- [ ] `scripts/check-public-api.sh` produces a clean diff (V16).
- [ ] `dotnet test tests/Platform.Architecture.Tests` passes with the new
      guards green and no existing guard removed (V17).
- [ ] `scripts/quality-gate.sh` passes end to end (V18), **including** the
      newly wired `verify-ui.mjs` step. If the verifier is not yet wired, that
      is an incomplete task, not an accepted gap — report it as outstanding.
- [ ] `git diff --check` is clean.
- [ ] Record every actual command output in the handoff. Report any
      environment-blocked check with its exact next action, never as passing.
- [ ] State the evidence boundary in `HANDOFF.md` explicitly: **no consumer
      repository is resolved, migrated or proven by this change.** The
      portability proof (V10) is the one thing that *does* resolve a feed
      outside this repository, and it is done with a fixture, not with
      `devloom` or `kairovia`.
