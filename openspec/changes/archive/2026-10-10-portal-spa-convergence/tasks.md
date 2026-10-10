# Tasks: Portal–SPA convergence

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/portal/model.rs` (`PortalSection`, `PortalSectionView`), `src/portal/sections.rs` (`build_section`, `controls_for`), `src/portal/render.rs`, `src/portal/activity.rs` (`render_dashboard_human`), `src/portal/portal_tests.rs` (struct literals + renderer needles), `tests/portal_contract.rs`, `frontend/index.html` (`<head>`), `docs/requirements-coverage.md` (§36 row), and the `control-plane-portal` canonical spec.
- [x] Map each of the twelve sections to its `spa_route` + `web_coverage` + note, its human lines, its JSON fields, its doc row and its contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight) and ownership (portal text projection only; no SPA/API/Core change).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/portal/model.rs`: add `spa_route()`, `web_coverage()`, `spa_note()` to `PortalSection` (pure twelve-arm tables) + `spa_route`/`web_coverage` fields on `PortalSectionView` with `#[serde(default)]`; contract version unchanged.
- [x] `src/portal/sections.rs`: populate both fields in `build_section`; `controls_for` untouched.
- [x] `src/portal/render.rs` + `src/portal/activity.rs`: per-section `spa:`/`web:` lines + dashboard header pointer line; exit codes, section ids, journaling unchanged.
- [x] `src/portal/portal_tests.rs`: update struct literals; add renderer needles.
- [x] `tests/portal_contract.rs`: pin all twelve `(section, spa_route, web_coverage)` JSON triples + human header/per-section pointers.
- [x] `docs/portal-spa.md`: twelve-row coverage table with CLI-only reason keys; `frontend/index.html`: one `<head>` comment noting the SPA is the single interactive surface.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify `forge portal` commands keep names/args/exit codes (help + typed-refusal tests green).
- [x] Verify catalog integrity suites green (every remaining honest-CLI pin still carries its reason; repair only what the suite names).
- [x] Verify existing suites green: `portal_cross_surface`, `forge_web_navigation_contract`, `web_command_reference_browser_contract`, `portal_ui_contract`, `forge_web_maintainer_surface_contract`, `cargo test --lib`.
- [x] Verify no SPA/API/Core byte change beyond the one HTML comment (diff review).
- [x] Verify every touched Rust file ≤1000 lines post-`cargo fmt`.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (record warnings).
- [x] `cargo test` green for: portal lib tests, new/updated `portal_contract` pins plus the regression list in §3. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS, `node scripts/check-spec-governance.mjs` PASS, and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files too.
- [x] Live proof: `forge portal dashboard|view` output shows SPA pointers per section; SPA loads with zero JS console errors (throwaway registry + real Chromium via bundled Playwright, scripts in `/tmp` — never committed).
- [x] `forge gate --dry-run` rehearse, then full `forge gate` with bounded timeout (e.g. `--timeout-secs 600`); record verdict with attribution (known pre-existing project-runtime 60s timeout; attributable blocks).
- [ ] Archive with `openspec archive portal-spa-convergence --yes` (no `--skip-specs`); promote the canonical spec; verify the canonical Purpose is source-backed (repair immediately if the archiver stamped TBD, before committing); update HANDOFF with evidence and pointer handling.
