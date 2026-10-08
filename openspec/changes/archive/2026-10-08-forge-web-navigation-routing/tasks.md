# Tasks: Real, deep-linkable dashboard navigation routes

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and delta specs agree, and a separate implementer could proceed without inventing architecture or an owner.
- [x] Read `src/web.rs` (`serve`, `serve_one`, `asset_name`, `content_type`, the unit test), `frontend/index.html` (sidebar nav, sections, base/links), `frontend/app.js` (`dashboardPage`, `openInWorkbench`, the two `#management` links), `scripts/web.sh` (`prepare_web_root`) and the house-style tests `tests/forge_web_maintainer_surface_contract.rs` / `tests/forge_web_workspace_onboarding_browser.rs`.
- [x] Confirm the routing decision (SPA path routing over separate pages) and record it in `design.md` §1.
- [x] Map each requirement and scenario to its `asset_name` arm, frontend route function, fleet-link change and contract-test assertion.
- [x] Add `tests/forge_web_navigation_contract.rs` with the design §7 scenarios.

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/web.rs`: teach `asset_name` the five dashboard paths (`/projects`, `/workbench`, `/management`, `/portfolio`, `/delivery`) as `index.html`; keep the allowlist exact and `/` at `login.html`.
- [x] `src/web.rs`: extend `static_server_has_an_allowlist_and_serves_login_at_root` to assert the new arms, the root login arm, and that unknown/traversal paths stay `None`.
- [x] `frontend/index.html`: add `<base href="/">`; wrap the projects content in `#view-projects`; add `#topbar-crumb`; give non-default view sections `hidden`; replace the sidebar `#` anchors with the real paths and `data-route`; balance the Data-sources section.
- [x] `frontend/app.js`: add `VIEW_BY_PATH` / `viewForPath` / `renderRoute` / `navigateTo`, the click interceptor, the `popstate` listener, and boot-time `renderRoute`; change the two `#management` links to `/management`; make `openInWorkbench` navigate to `/workbench`.
- [x] `frontend/styles.css`: only if an explicit view-hidden rule is required — not required (`hidden` boolean attribute hides views natively; no stylesheet change).
- [x] Implement the delta spec's scenarios alongside the code above.

## 3. BFS — Cross-surface regression and completeness

- [x] Verify `/`, `/login.html`, `/index.html`, `/styles.css`, `/config.js`, `/app.js` are byte-unchanged responses.
- [x] Verify auth/session behavior and the login redirect are unchanged.
- [x] Verify no JSON API, registry/journal schema, catalog row/count or CLI path changed.
- [x] Verify no `#` fragment remains as a navigation mechanism (sidebar, fleet "Manage", workspace hint) while the in-page skip links remain for accessibility.
- [x] Verify `scripts/web.sh` needs no change (its symlink staging picks up frontend files) and that a deep link reloads to its view.
- [x] Remove any current-change placeholder.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test --test forge_web_navigation_contract` green (new).
- [x] `cargo test --lib command_catalog`, `cargo test --bin forge`, `--test plugins_contract`, `--test catalog_contract`, `--test classify_derive_contract`, `--test classify_apply_contract`, `--test web_login_credentials_contract`, `--test forge_web_command_catalog_contract`, `--test forge_web_maintainer_surface_contract` green. Record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [x] `git diff --check` clean; review newly added files.
- [ ] `forge gate --dry-run`, then `forge gate --timeout-secs 600` with the verdict recorded in HANDOFF; no new failure attributable to this change.
- [ ] Archive with `openspec archive forge-web-navigation-routing --yes` (no `--skip-specs`); author the promoted spec's Purpose; update HANDOFF with the evidence and pointer handling.
