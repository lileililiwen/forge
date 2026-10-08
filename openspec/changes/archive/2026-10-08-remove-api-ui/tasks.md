# Tasks: Remove the in-process /ui HTML portal

## 1. BFS — Baseline and impact coverage

- [x] Read every file involved: `src/api/ui/*`, the OIDC sections of `src/main.rs` and `src/api/admin.rs`, the named test files, the named specs.
- [x] Map every consumer of `crate::api::ui::*` to its site; confirm it is only `src/api/mod.rs`'s route table and the deleted tests.
- [x] Confirm the OIDC machinery in `src/identity/` is also consumed by the `forge identity` CLI (which stays) — do not remove the OIDC code itself.
- [x] Add the OpenSpec change directory with proposal, design and a `remove-api-ui` delta spec.

## 2. DFS — File-by-file removal

- [ ] Delete `src/api/ui/` entirely (5 Rust files plus `static/`).
- [ ] Drop the `mod ui;` declaration (or equivalent) and any `use crate::api::ui::*;` imports in `src/api/mod.rs`.
- [ ] Drop the OIDC sign-in route registrations in `src/api/mod.rs` (the ones that mounted `/ui/sign-in`, `/ui/auth/callback`, `/ui/sign-out`); the routes are simply absent.
- [ ] Slim `tests/identity_contract.rs` to drop the OIDC round-trip cases that hit the deleted routes; keep the `forge identity init`, `forge identity validate`, and other non-`/ui/*` cases.
- [ ] Delete the named test files (`tests/portal_ui_contract.rs`, `tests/portal_browser_a11y.rs`, `tests/forge_portal_frontend_contract.rs`).
- [ ] Delete the `/ui/*` cases from `tests/portal_contract.rs` and `tests/portal_cross_surface.rs`; if no cases remain, delete the files outright.
- [ ] Delete the three named specs from `openspec/specs/`: `forge-web-human-dashboard`, `forge-web-workspace-onboarding`, `forge-admin-login`.

## 3. BFS — Cross-surface regression and completeness

- [ ] `forge web serve` on port 4173 still loads and exercises the JSON API on port 8765.
- [ ] `forge api serve` on port 8765 still serves `/healthz` and `/v1/...` routes.
- [ ] `forge portal dashboard` and `forge portal view` still work (they were not removed).
- [ ] `forge identity init`, `forge identity validate`, `forge identity challenge`, `forge identity callback`, `forge identity list`, `forge identity inspect`, `forge identity terminate` all still work.
- [ ] `GET /ui/*` returns 404 on the API listener.
- [ ] No public symbol unrelated to the removed surface is touched.

## 4. Verification

- [ ] `cargo fmt` then `cargo fmt --check` clean.
- [ ] `cargo build --workspace` 0 errors.
- [ ] `cargo test --workspace` — every surviving test is green; deleted tests are no longer in the output.
- [ ] `node scripts/check-openspec-change-names.mjs` PASS and `openspec validate --all --strict --no-interactive` 0 failures with this change active.
- [ ] `git diff --check` clean; review newly added files.
- [ ] `forge gate` local run with verdict recorded; the `source-file-size` count should drop by the line count of the deleted files; the `forge-web-*` specs disappear from the count of canonical specs.
- [ ] Archive with `openspec archive remove-api-ui --yes` (no `--skip-specs`); promote the canonical spec; update HANDOFF with the evidence and pointer handling.
