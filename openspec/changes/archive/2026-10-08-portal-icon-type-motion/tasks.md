# Tasks: Portal icon type motion

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta spec agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Map each requirement/scenario to its `frontend/` symbol (icon glyph sites in `index.html`/`login.html`/`app.js`/CSS, type rules, motion tokens in `styles.css`) and test assertion.
- [x] Read `frontend/index.html` icon sites, `frontend/login.html` story/lock/submit sites, `frontend/app.js` caret + submit-restore sites, `frontend/styles.css` type/motion rules, and `tests/forge_web_navigation_contract.rs` token assertions.
- [x] Record pre-change icon/type/motion behavior (glyph inventory, undeclared base, bare 120ms-everywhere) without changing live data.
- [x] Verify the implementation-handoff gate: language (vanilla HTML/CSS/JS, no build), project (vendored `frontend/`), shared-library (none — project-local), UI/UX boundary (icons, type, motion tokens only).

## 2. DFS — Requirement-by-requirement implementation

- [x] `frontend/index.html`: 5 nav + 1 search + 8 summary + 2 empty-mark glyphs → inline SVG from the single stroke set, `aria-hidden` kept, labels/`aria-current` intact.
- [x] `frontend/login.html`: 3 story checks → `.check-disc` inline SVGs; lock `●` → lock SVG; submit `→` → arrow SVG.
- [x] `frontend/app.js`: caret `▸` → chevron SVG `innerHTML`; login submit restore string uses the same arrow SVG; no router/fetch/payload change.
- [x] `frontend/styles.css`: three base edits (`li:before` glyph removed, `.wb-digest` break-all → anywhere, lock-icon box sizing) + appended slice-5 block (icon/type/motion tokens and wiring at unchanged computed values except intended line-height/prose/motion upgrades).
- [x] `tests/forge_web_navigation_contract.rs`: new static token test for slice 5.
- [x] Delta spec scenarios implemented alongside the code above; payloads/endpoints unchanged.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify deep-link reload/back-forward/login behavior unchanged; slice-1 focus (`lastRouteView`, `#main-content`), scroll-padding, dark-scheme pairing, contrast pairs and slice-2 targets/insets/dvh/press-band and slice-3 labels/summaries/toggle and slice-4 breakpoints/nav-scroll/filter-state/unknown/z-scale untouched.
- [x] Verify no JSON API, registry/journal, catalog row/count, CLI, or static-server change; slice-6 surfaces untouched; no external font/CDN fetch.
- [x] Verify `cargo fmt --check`, `cargo build`, `forge_web_navigation_contract` + `forge_web_manage_deep_link_browser` + `forge_web_workbench_deep_link_browser` green with actual counts.
- [x] Verify static token assertions over shipped files (icons, type, motion, prior-slice tokens intact) and record the table.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors (pre-existing warnings only).
- [x] `cargo test --test forge_web_navigation_contract` green; `cargo test --test forge_web_manage_deep_link_browser` green; `cargo test --test forge_web_workbench_deep_link_browser` green. Record actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [x] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [x] Archived `portal-icon-type-motion` via `openspec archive --yes` (no `--skip-specs`) and committed: implementation, archive+spec promotion, HANDOFF evidence in follow-up commits; `openspec list` reports no active changes.
