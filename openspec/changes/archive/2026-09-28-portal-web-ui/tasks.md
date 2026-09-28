# Tasks: portal-web-ui

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the existing `forge api serve` listener, `authorize()` flow, `MAX_BODY_BYTES`, idempotency, and confirm-gate pattern are the seams the new UI hooks into; record the renderer choice (`maud`) and the exact three-crate dep closure.
- [x] 1.2 Confirm the v0 scope reduction: per-row live liveness is out of scope (CLI/API remain the surfaces for SSH-bound probes); the list page consumes the registry's latest journal row state instead.
- [x] 1.3 Confirm the auth/CSRF model: bearer token re-checked on the POST, cross-origin POST refused via the `Origin` header; existing `policy::redact_credentials` continues to scrub journal strings before render.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add `maud` (and the two transitive companions) to `[workspace.dependencies]`; register the three crates in `deny.toml` with reasons; run `cargo deny check` and `cargo build` to confirm the closure opens cleanly.
- [x] 2.2 Add `src/api/ui/render.rs` (maud templates + layouts: list, detail, plan-preview, operation-accepted, error pages); unit-test the escape matrix on adversarial project names / notes / evidence.
- [x] 2.3 Add `src/api/ui/auth.rs` (bearer-token re-check, cross-origin `Origin` check, idempotency-key dedupe); unit-test the rejection paths.
- [x] 2.4 Add `src/api/ui/routes.rs` (the three handlers: `GET /ui`, `GET /ui/projects/{id}`, `POST /ui/projects/{id}/publish`); wire them into `src/api/mod.rs` through a new `Route::UiFleet`, `Route::UiProjectDetail { id }`, `Route::UiProjectPublish { id }` variant dispatched by the existing router.
- [x] 2.5 Add typed HTML error pages for 400/401/403/404/409/500 inside `src/api/ui/render.rs` (the dedicated `src/api/ui/error.rs` plan task is folded into the render module because error pages share the maud `<style>` block — extracting a one-screen module would force re-export of the stylesheet across files). Unit-tested via the `forge_error_to_html` mapping in routes and the 401/404/403 contract tests.
- [x] 2.6 Add `tests/portal_ui_contract.rs` HTTP contract tests (no browser engine): list row count equals fixture roster; detail 404s unknown ids; publish without confirm returns the plan page and enqueues nothing (journal diff zero); publish with confirm returns 202 + journal row appears; project-controlled strings arrive escaped; cross-origin POST is refused; bearer-token re-check is enforced.
- [x] 2.7 Confirm the existing API JSON envelopes, CLI outputs, MCP `tools/list`, and portal CLI views stay byte-identical (content negotiation: `Accept: application/json` returns the unchanged envelope; CLI/MCP code paths are not touched).

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 `cargo fmt --check` (touched files), `cargo build`, `cargo clippy --all-targets -- -D warnings` (touched files), `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain` (full suite green + new suites); `git diff --check` pass.
- [x] 3.2 `cargo deny check` green with the three new entries; the rest of the closure unchanged.
- [x] 3.3 `node scripts/check-openspec-change-names.mjs` and `openspec validate --all --strict --no-interactive` (0 failed).
- [x] 3.4 Document the v0 scope reduction in HANDOFF: per-row live liveness in the list is out of scope for this change; operators keep using `forge fleet online` for live liveness until a future package extends the list page.

## 4. Verification

- [x] 4.1 Re-render the proposal/design/spec/tasks against `workflow.md` BFS→DFS→BFS, the capability section, and the non-goals section. Confirm the maud choice, the v0 scope reduction, and the auth/CSRF model are consistent across all four files.
- [x] 4.2 Live evidence: `curl` capture of the three pages against `forge api serve --bind 127.0.0.1 --port 18765` with a fixture registry (operator-runnable, no Mac required); pages return 200 HTML, plan preview is a dry-run, republish-with-confirm journals a row.
- [x] 4.3 Archive without `--skip-specs`; promote `portal-web-ui` to the canonical spec; advance the HANDOFF pointer to the next active eligible change.