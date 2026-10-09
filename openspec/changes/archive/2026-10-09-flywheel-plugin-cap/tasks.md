# Tasks: Flywheel plugin cap grouping

## Phase 1 — BFS baseline

- [x] 1.1 Run `node scripts/check-openspec-change-names.mjs` PASS; confirm `main` at `f435f3b` clean; `openspec list` shows only `github-gh-fallback-register` active.
- [x] 1.2 Map touch points: `src/plugins/mod.rs` vocabulary/kinds/routing, `src/cli/commands.rs` + `src/main.rs` flat commands, `src/cli/plugins.rs` list shape, `src/portal/sections.rs` projects controls/entries, `frontend/app.js` rail/delivery/projects, `frontend/index.html` shells, `tests/portal_ui_contract.rs`, `tests/browser/lifecycle-rail-check.mjs` + `tests/lifecycle_rail_browser.rs`, `docs/` demo slot, `kits/manifest.json` feed shape.
- [x] 1.3 Set `current_spec: flywheel-plugin-cap` in HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `src/plugins/mod.rs`: extend `CAPABILITIES` (+`gate,quality,agent,contract,analytics`), `PluginKind` (+`Gate,Quality,Agent,Contract` with `as_str`/`parse`), builtin table + `kits/manifest.json` first-read with `providers.yaml` override, `gate/quality/agent/contract_plugins()` beside `metadata_plugins()`; keep `Metadata|Delivery` + `Invalid` semantics; lib tests green.
- [x] 2.2 Cap CLI: `CapCommands` + `src/cli/cap.rs` (`list|inspect|add|run`, `CAP_GROUPS` mapping gate/agent/contract/delivery per brief) + `Commands::Cap` dispatch; `cap list` from `CAPABILITIES` + live `plugins list` states; old flat commands untouched.
- [x] 2.3 Portal: projects `controls_for` gains `forge cap list|inspect`; `build_projects_section` gains `cap_group` attribute.
- [x] 2.4 Web flywheel: `#wb-idea-entry` (graduation preview/import + studio spec link, `?project=`+`?step=`), `#delivery-next-idea` (Next-idea prompt + `&step=idea` link + maintain refresh shortcut), `#cap-filter` projects filter + `#cap-badge` rail badge; appended CSS only; `node --check` clean.
- [x] 2.5 `docs/flywheel-demo.md`: hookit idea→scaffold→gate→publish→maintain 5 steps with exact CLI + `?project=&step=` web URLs.
- [x] 2.6 Tests: `tests/portal_ui_contract.rs` +5; `tests/browser/lifecycle-rail-check.mjs` + step0/cap-list/demo-URL notes; `tests/lifecycle_rail_browser.rs` doc + assertion passthrough.

## Phase 3 — BFS regression/completeness

- [x] 3.1 `cargo test --test portal_ui_contract --test lifecycle_rail_browser` green; `cargo test --lib plugins::` green; flat CLI `--help` unchanged (spot: `gate`, `agent`, `contract`, `delivery` still list).
- [x] 3.2 `forge cap list --format json` derives from `CAPABILITIES` + live states; `forge plugins list` still honest-empty without config; unknown kind/capability still `Invalid`.
- [x] 3.3 No new frontend dep (`import(`/`require(` absent, 2 scripts intact); rail still 8 steps/one current/one Tab stop; `?step=bogus` ignored.

## Phase 4 — Verification

- [x] 4.1 `cargo fmt --check` / `cargo build` clean (pre-existing warnings only).
- [x] 4.2 `cargo test --test portal_ui_contract --test lifecycle_rail_browser` recorded.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` PASS; `git diff --check` clean.
- [x] 4.4 `forge gate --dry-run` rehearsed + bounded `forge gate --timeout-secs 500`; verdict recorded in HANDOFF (attributable blocks, pre-existing recorded).
- [x] 4.5 Archive `flywheel-plugin-cap` without `--skip-specs`; advance `current_spec` back to `github-gh-fallback-register`.
