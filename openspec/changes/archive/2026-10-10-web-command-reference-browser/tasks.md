# Tasks: Web command-reference browser

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `frontend/index.html` (`#view-projects` structure), `frontend/app.js` (`loadCommands`, `catalogCommands`, `AVAILABILITY_LABELS/BADGE`, `CATEGORY_LABELS`, `el()`, clipboard fallback, router allowlist), `frontend/styles.css` (badge/panel/result tokens, 12px floor block), `src/api/command_catalog/` (row shape, reason invariants, web-route list) and the `portal-web-ui` canonical spec as the reuse contract.
- [x] Map each requirement and scenario to its HTML ids, app.js function, CSS class and contract-test assertion.
- [x] Record toolchain (Rust stable, Node for name preflight) and ownership (frontend-only; no Rust file touched).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/index.html`: add the `#command-reference` section (heading, description, search + availability controls, live count, copy-status region, list container) inside `#view-projects` after the project panel.
- [x] `frontend/app.js`: add `referenceViewForRoute`, `renderCommandReference` (filter + row render + empty states + copy with clipboard/select fallback), wire controls + `loadCommands` arms; non-web rows render badge + reason + exact CLI + copy only; web rows render view link + CLI text.
- [x] `frontend/styles.css`: one additive block for the reference section/rows/controls at the 12px floor, reusing badge/panel tokens.
- [x] `tests/web_command_reference_browser_contract.rs`: static-token oracle per design §8.
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify no API/CLI/router/catalog/registry/journal change (`git status` shows only frontend + test + change package).
- [x] Verify `src/web.rs` allowlist, sidebar nav, `VIEW_BY_PATH` and all existing frontend tests untouched and green.
- [x] Verify every non-web availability state renders badge + reason + CLI + copy with no executable control (grep for new fetch/POST/eval/innerHTML: none).
- [x] Verify web rows link only to the five real paths and hard-load safely (shell serves them).
- [x] Verify a11y contracts: native controls, live count + copy status, keyboard operability, 12px floor, no new motion.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (no new warnings).
- [x] `cargo test` green for: new `web_command_reference_browser_contract`, `portal_ui_contract`, `forge_web_command_catalog_contract`, `forge_web_navigation_contract`. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] Browser oracle (throwaway listeners + sign-in): reference renders all rows, search + availability filter work, badge/reason/CLI/copy on non-web rows, view links on web rows, zero JS console errors. Record evidence.
- [x] `forge gate --dry-run` rehearse, then full `forge gate` bounded (e.g. `--timeout-secs 600`); verdict recorded; no new failure attributable to this change.
- [ ] Archive with `openspec archive web-command-reference-browser --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with evidence and pointer handling.
