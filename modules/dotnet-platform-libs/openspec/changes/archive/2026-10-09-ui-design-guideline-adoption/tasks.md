# Tasks: DESIGN.md adoption, contrast record, Tailwind outputs, kit 0.2.0

Ordered BFS → DFS → BFS: measure → change the source and projections → re-audit
callers, feed, guards, docs.

## 0. Baseline (BFS)

- [x] Measure every text-carrying and non-text semantic pair in both themes (D-3 table);
      record before/pass/fail per pair.
      Done: every text pair measured ≥ 4.5:1 and both non-text pairs (border, focus
      ring) ≥ 3:1 in light and dark; zero primitive hexes needed adjustment. The
      complete measurement record is design.md D-3 and `docs/ui-tokens.md`.
- [x] `node /home/paul/code/forge/scripts/check-openspec-change-names.mjs` and the same
      rule applied to this repository's active changes: name legal.
      `check-openspec-change-names: PASS`; `ui-design-guideline-adoption` matches
      `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`.
- [x] Record pre-change feed digests and tokens.css sha (the values `UiDistributionTests`
      re-pins at the end).
      Pre-change at HEAD: tokens.css sha `2216411d7e42ef5399c79c0b84e34460cff97251aa1fa47a09870f455be790f3`; feed 0.1.0 tarballs
      design-tokens `2b5f2c73af8deb20f753faa7421a15fefac1ba3f64aaa895bc960786964226be`, react-ui `45c757f6d222952cbeebb058670b9d731472b0afe776a3a96a23cc479ff59809`, react-shell `0a23272e4f744712f2757e2c0be62b55a0f9fa93edeb7dddd22910e5635cba52`,
      react-auth `5ea55e34bcadcd8147bfef0eca9a059c8f1540880921506a73681be05fb1ee0e`, react-templates `079a906376a3af2568de34555e965880bbb8e61c948464e822943341bf4e07d7`. Post-change: tokens.css
      sha `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b`; 0.2.0 tarballs design-tokens `e48284cbc105cd6ded61b2d6320cc38d109a5f0306d565e4f9c1b07dd91cbc8c`, react-ui `cfa76744ae7b0e57b187f5d405566ea7256ba5b84a84599eccccafab246097cf`,
      react-shell `be0666206873a5ce62dee020bfae78298563aecf85ce84f99839c2b6ebaafd6e`, react-auth `0f1bfd63dacd27385d09974dd7dab5df2fac6881eeeaac95d34c3e48da8d0732`, react-templates `86b9e8888cc3ee993a3ed0205ef4d66336efd7abda0ae1fe04f0cab9d785a8d0`.

## 1. Token source (DFS)

- [x] `font.sans` → `Outfit, system-ui, sans-serif` with consumer-font-loading note
      (D-1); rejected guideline values recorded as deviations (D-2).
      `$description` on `font.sans` records both: loading Outfit is the consumer's
      job; the 14px base and 2–10px spacing are the documented rejections.
- [x] `color.focusRing` bridge token → `--color-focus-ring` aliasing
      `{semantic.color.focusRing}` (D-4); `$description`s on the audited tight-margin
      primitives (D-3).
      Emits `--color-focus-ring: var(--focus-ring)` and resolves to `#1d4ed8` in
      `tokens.ts`; green.700 and amber.700 now carry their measured 4.58:1 muted-chip
      margins so a future shade change is warned by data, not memory.
- [x] Re-baseline `emitted-baseline.json`: exactly the `--font-sans` entry, with this
      change named as the record (D-7).
      Diff shows one changed token entry (38 pinned names before and after), plus
      `rebaselinedAt` and a `$description` naming this change as the authorized
      contract record.

## 2. Generator projections

- [x] `generate-tokens.mjs` writes `dist/tailwind/preset.js` (v3 ESM preset, var-backed)
      and `dist/tailwind/theme.css` (v4 `@theme inline`) from `tokens.json` only (D-5).
      Generator reports `14 color/1 font/6 spacing/4 radius Tailwind mappings`;
      no path list or value is hardcoded in the new block.
- [x] Regenerate all dist artifacts.
      Re-runs are byte-identical; shas as recorded in §0.

## 3. Verification surface

- [x] `verify-ui.mjs`: required artifacts += the two Tailwind outputs; required-token set
      += `--color-focus-ring`; new `ui-tailwind-incomplete` check (missing semantic name
      or leaked literal in either output).
      The new check covers every semantic color key in both outputs, the
      `--color-focus-ring` citation, and rejects any hex/`rgb(a)` literal that slips
      into the Tailwind outputs.
- [x] `selftest-verify-ui.mjs`: fixture copies += Tailwind outputs; negative case proving
      `ui-tailwind-incomplete` fires.
      Two negative cases added (dropped preset mapping; `#ffffff` in a `@theme` line);
      self-test now runs 1 positive + 18 negative cases, all producing their named
      diagnostics.
- [x] `node ui/scripts/verify-ui.mjs` and `node ui/scripts/selftest-verify-ui.mjs` pass.
      `UI verification passed: 13 required artifacts …; 50 declared tokens emitted
      (50 in tokens.css, 38 baselined entries); Tailwind preset and theme block map
      every semantic color with no literal; …` and
      `verify-ui self-test passed: 1 positive control and 18 negative cases`.

## 4. Feed and kit version

- [x] Bump all five `ui/packages/*/package.json` to 0.2.0 (versions + internal dependency
      ranges).
      `pack-ui.mjs` enforces one kit version across the feed (D15), so all five moved;
      design.md D-6 records why a single-package bump was not possible.
- [x] `node ui/scripts/pack-ui.mjs`; old 0.1.0 tarballs removed; `feed.json` regenerated
      with the new digests and the `dist/tailwind/*` members.
      design-tokens members=5 including `package/dist/tailwind/preset.js` and
      `package/dist/tailwind/theme.css`; the five 0.1.0 tarballs are deleted.
- [x] `node ui/scripts/verify-ui-feed.mjs --self` passes; determinism re-checked
      (pack twice, same sha).
      `ui-feed self-check passed: 5 committed tarballs match ui/feed/feed.json at kit
      version 0.2.0`; re-packing reproduces the digests in §0 byte for byte.

## 5. Consumers inside this repository

- [x] `Platform.UI.Razor/wwwroot/platform-tokens.css` value-sync: Outfit stack,
      `--color-focus-ring` added (D-8).
      Light `#1d4ed8`, dark `#bfdbfe`, matching the generated kit's resolved values;
      palette unchanged and the razor NuGet package version untouched (0.1.0, 79
      packages, per `UiDistributionTests`).
- [x] `ui/tests/conformance-fixtures/feed-consumer/package.json` and
      `scripts/test-ui-distribution.sh` moved to 0.2.0 (fake-mismatch version becomes
      0.3.0); `scripts/test-ui-distribution.sh` passes end to end.
      `ui-distribution test passed: 19 cases.` including the offline `npm install`
      portability proof at an unrelated path.
- [x] `tests/Platform.Architecture.Tests/UiDistributionTests.cs`: re-pin tokens.css sha,
      kit/package version 0.2.0, guard name and comments updated; `dotnet test` of the
      architecture suite passes.
      Re-pinned to `7ca6b7956b3f6b475254b4a5d0cf599724e4e1af5f0f88aba4fbccb9f38fc19b` / `0.2.0`, renamed
      `Ui_npm_packages_ship_0_2_0_with_digest_pinning`, and added feed-member
      assertions for both Tailwind outputs; architecture suite: 383 passed, 0 failed;
      full-solution `dotnet test -c Release` passes inside `quality-gate.sh`.

## 6. Documentation

- [x] `docs/ui-tokens.md`: adopted accessibility rules (AA targets, focus indicator,
      seven component states), full contrast table with measurements, deviations from
      DESIGN.md, Tailwind usage (v3 preset + v4 theme, alpha limitation, import order),
      consumer font-loading note.
- [x] `ui/README.md`: version policy (0.2.0 and why), layout table, emitted-name counts,
      baseline re-record, focus-ring alias, Tailwind outputs; root `README.md` and
      `docs/packages.md` version references updated.

## 7. Regression BFS + gates

- [x] `openspec validate ui-design-guideline-adoption --strict` passes before archive.
      `Change 'ui-design-guideline-adoption' is valid`; full
      `scripts/quality-gate.sh` also completed with exit 0 before archive.
- [x] `git diff --check` clean; package manifest unchanged (`generate-package-manifest.sh
      --check`); `check-public-api.sh` untouched/passing.
      `Manifest matches source.`; no `.cs` public surface changed.
- [x] Archive the change (`openspec archive -y`), promoting both spec deltas;
      `openspec validate --specs --strict` passes.
      Both deltas promoted (5 ADDED + 1 MODIFIED requirement); post-archive spec
      validation is part of the same `quality-gate.sh` run pattern.
