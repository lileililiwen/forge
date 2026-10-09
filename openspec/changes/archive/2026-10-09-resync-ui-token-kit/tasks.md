# Tasks: Resync the vendored UI token kit mirror to kit 0.2.0

## Phase 1 — BFS baseline

- [x] 1.1 `node scripts/check-openspec-change-names.mjs` PASS before selection;
      record foreign uncommitted paths (`src/api/*`, HANDOFF pointer line,
      `openspec/changes/workbench-health-latency/`) as never-stage set.
- [x] 1.2 Map the mirror surfaces: `kits/tokens/*`, `kits/scripts/verify-tokens.mjs`,
      `kits/manifest.json`, `src/kit/assets/verification.rs::token_assets`,
      `src/kit/assets/model.rs::KitManifest`, `src/kit/registry.rs::PLATFORM_UI_KIT_VERSION`,
      `src/contract/mod.rs` inventory row + `inventory_agreement`,
      `tests/kit_contract/{tokens_and_assets,upgrade,tampering,manifest,feed}.rs`;
      confirm no npm-tarball or Tailwind consumer exists in `src/`, `tests/`, `kits/`.
- [x] 1.3 Set `current_spec: resync-ui-token-kit` in HANDOFF (worktree-only
      during the cycle; the foreign uncommitted pointer line is preserved and
      restored, never committed).

## Phase 2 — DFS requirement implementation

- [x] 2.1 Copy `ui/packages/design-tokens/dist/tokens.css` and `tokens.ts` from
      kit revision `6bf3946…` into `kits/tokens/`; verify byte-identity with
      `diff` and measure sha256 (`7ca6b795…`, `7e0633d3…`).
- [x] 2.2 Re-derive `kits/scripts/verify-tokens.mjs` from upstream
      `verify-ui.mjs` at `6bf3946…`: provenance comment to the new revision,
      add `--color-focus-ring`, add `tokenValues` and `semanticColorValues`
      (light+dark) assertions on the vendored pair; run it green.
- [x] 2.3 Update `kits/manifest.json`: `revision` → `6bf3946…`, add `version`
      `"0.2.0"`, new per-file sha256 for the three UI files, `synced_at` now;
      feed entries untouched.
- [x] 2.4 Model `version` in `KitManifest` (`src/kit/assets/model.rs`,
      `#[serde(default)]`) so `forge kit pack` preserves it.
- [x] 2.5 `src/kit/assets/verification.rs::token_assets()`: new digests;
      `src/kit/registry.rs`: `PLATFORM_UI_KIT_VERSION` → `"0.2.0"`.
- [x] 2.6 `src/contract/mod.rs`: inventory row version `"0.2.0"`;
      `inventory_agreement` checks the UI-kit row against the compiled-in
      constant, other rows keep the `0.1.0` baseline.
- [x] 2.7 `tests/kit_contract/tokens_and_assets.rs` and
      `tests/kit_contract/upgrade.rs`: `platform-ui-web@0.1.0` literals and
      rendered-version assertions → `0.2.0` (incl. the stale-pin `replacen`
      fixture).

## Phase 3 — BFS regression/completeness

- [x] 3.1 `cargo test --test kit_contract` green (tokens_and_assets, upgrade,
      tamper round-trip, manifest, feed, registration, classification).
- [x] 3.2 `cargo test --lib contract::` green; `forge contract list` renders
      the bumped row.
- [x] 3.3 `node kits/scripts/verify-tokens.mjs` PASS; repo-wide grep shows no
      surviving `720d0bc/98dfd64/c740bd9` reference outside history; no
      `platform-ui-web@0.1.0` left in `tests/`.

## Phase 4 — Verification

- [x] 4.1 `cargo build` + `cargo fmt --check` clean.
- [x] 4.2 `node scripts/check-openspec-change-names.mjs` PASS;
      `openspec validate --all --strict --no-interactive` PASS.
- [x] 4.3 `git diff --check` clean; staged set excludes foreign `src/api/*`
      paths and the foreign workbench change folder.
- [x] 4.4 `forge gate --dry-run` rehearsed; bounded full `forge gate` run;
      verdict (or exact blocked command + next action) recorded in HANDOFF.
- [x] 4.5 Archive `resync-ui-token-kit` without `--skip-specs`; commit 1
      explicit paths; HANDOFF evidence commit 2; restore foreign pointer line
      uncommitted; stop, no push.
