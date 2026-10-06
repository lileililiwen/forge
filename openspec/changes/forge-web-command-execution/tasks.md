## Task 1 — Backend: typed admin execution routes and catalog reclassification

- [ ] Export shared route constants for the two actions and register `POST /v1/admin/projects/{id}/feature` and `/spec` (plus their `OPTIONS` `AdminOptions` pairs) in the routing table and the admin session-gate allowlist, mirroring `AdminProjectApply`.
- [ ] Implement the two-step preview + confirm/digest guard in `admin.rs`: a no-`confirm` request returns a bounded preview plus `plan_digest` via the existing `plan_digest` helper and performs no write; a `confirm: true` request recomputes the digest and only on a match delegates to `handle_add_feature` / `handle_generate_spec`. Verify an unconfirmed or wrong-digest request is refused with no write and a valid one produces the same result as the CLI.
- [ ] Reclassify the `feature add` and `spec generate` catalog rows from `not_yet_web` to `web` with their real routes, add the routes to `IMPLEMENTED_WEB_ROUTES`, and update the catalog contract test's expected `web`/`not_yet_web` counts.
- [ ] Add a Rust contract test (`tests/forge_web_command_execution_contract.rs`) covering per-action delegation, session-gate refusal, no-confirm refusal, wrong-digest refusal (no write), and the two catalog rows reporting `web`; run `cargo build`, `cargo test` and `cargo clippy` and record the actual results.

## Task 2 — Frontend: browser preview-and-confirm workflow and handoff

- [ ] Add the two action forms in `frontend/app.js` within the project workbench panel (feature id + optional version; finding ids + optional reason): request the preview, render the bounded summary + digest, then submit `confirm: true` + the digest and render the typed result through existing safe DOM builders; no free-text shell field.
- [ ] Surface admin route error codes/messages without leaking paths or secrets; keep exact-origin request handling and the existing accessibility/DOM conventions.
- [ ] Verify in a headless browser against a real registry: sign in, preview and confirm both actions end-to-end, and confirm the catalog presents them as runnable; record the observation.
- [ ] Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and `git diff --check`; confirm the untouched bearer `/v1` routes, CLI, workbench, delivery and portfolio behavior regress clean.
- [ ] Archive the verified change with promoted canonical specs (never `--skip-specs`), update HANDOFF with the delivery evidence, advance or remove the `current_spec` pointer, and commit the handoff only.
