# Tasks: forge-portal-identity-rotation

## 1. BFS — Baseline and impact coverage

- [x] Confirm proposal, design and delta spec agree, and a
      separate implementer could proceed without inventing
      architecture or an owner.
- [x] Read `src/cli/commands_services.rs:44-49`,
      `src/cli/identity.rs:57-78` (the CLI source of truth for
      the rotation contract), `src/identity/global.rs:172-218`
      (the Core function the new route delegates to — Argon2id
      hash, 12–1024 character length, unconfigured-registry
      refusal, `revoke_all_sessions_in` cascade), `src/api/admin/agent_identity.rs`
      (the new-file-beside-table precedent; `router.rs` is at
      999/1000 lines so the new handler is matched beside the
      table), `src/api/admin/gateway.rs:512-644` (the global
      `guarded` / `cors` / `error` envelope the new handler
      reuses verbatim), `src/api/model.rs:509-528` (the
      `Route` enum shape), `src/api/router.rs` (the new arm
      placement, the `required_permission` / `admin::handle`
      / exhaustiveness list, the 999/1000 cap), `src/api/admin/mod.rs:26-32`
      (the `route_beside` chain), `src/api/admin/deploy.rs`
      (the per-route dispatch), `src/api/command_catalog/routes.rs`
      and `rows_platform.rs` (the `web_at` conversion + the
      `REASON_TTY_HIDDEN` fallback), `frontend/index.html:24-27`
      (the topbar `Sign out` button), `frontend/app.js:340-405,
      :3540-3560, :3495-3529, :6394-6399` (the login form +
      per-field-error + sign-out patterns), and the existing
      contract tests `tests/forge_admin_api_contract.rs` +
      `tests/web_shipping_provider_reads_contract.rs` +
      `tests/web_login_credentials_contract.rs` for the test
      shape.
- [x] Map each requirement/scenario to handler, route arm,
      catalog row, topbar control and contract-test
      assertion; record toolchain and ownership (new
      `src/api/admin/identity.rs` + one route constant + one
      `Route` variant + one catalog row conversion + one
      topbar button + one dialog + one contract test; no
      CLI/Core/schema change; `router.rs` stays ≤ 1000 lines).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/api/admin/routes.rs` + new `src/api/admin/identity.rs`:
      one route constant + matcher + one session-gated POST
      handler reusing `guarded` / `cors` / `error` from
      `gateway.rs` and `forge::identity::global::change_password`
      from `identity/global.rs`; the typed-refusal envelope
      (no echo of any password field); every response carries
      `contract: API_CONTRACT_VERSION`. No CLI change, no
      Core change, no schema change.
- [x] `src/api/model.rs` + `router.rs` + `admin/mod.rs` +
      `admin/deploy.rs`: one `Route` variant
      (`AdminIdentityChangePassword`); one arm in
      `route_request` matching
      `("POST", ["v1", "admin", "identity", "change-password"])`
      placed before the generic `["v1","admin",_]` OPTIONS
      wildcard; one arm in `required_permission` (returns
      `None` — same posture as `AdminSessionGet` /
      `AdminSessionPost`); one arm in the `admin::handle`
      short-circuit; one arm in the `not_found` exhaustiveness
      list; `pub mod identity;` and a chain in `route_beside`
      before `creation`; one dispatch arm in `deploy.rs`.
      `router.rs` verified ≤ 1000 lines.
- [x] `src/api/command_catalog/routes.rs` +
      `rows_platform.rs`: one `WEB_ROUTE_ADMIN_*` constant +
      one `IMPLEMENTED_WEB_ROUTES` entry; the
      `identity.change-password` leaf converts to
      `web_at(ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD)`; the
      row's reason keeps the TTY/secret call-out as a
      fallback. The catalog count pin stays the same
      (conversion, not addition).
- [x] `frontend/index.html` + `frontend/app.js`: one
      topbar `Change password` button next to `Sign out`;
      one `<dialog>` with three password fields, one
      confirmation checkbox, one submit button, one
      `role=status` result region, one error summary; the
      click handler binds the button, validates inline,
      posts to the named route, auto-signs-out on success
      and renders typed refusals per field via the
      existing `setFieldError` / `renderErrorSummary` /
      `clearErrorSummary` helpers; no `innerHTML`, no
      `eval(`, no `Function(`, no inline script, no new
      dependency, no `XMLHttpRequest`, no shell.
- [x] `tests/web_portal_identity_rotation_contract.rs`:
      design §7 oracle (static asset pins, route contract
      through `forge::api::handle` — anon 401, wrong
      content-type 415, oversized 413, weak 400,
      mismatch 400, current-incorrect 403, unconfigured
      403, success 200 with `changed: true` and the
      pre-rotation session invalidated; mutation-guards
      on every refusal that the hash and pre-rotation
      session survive the refused call; catalog pin).
- [x] Delta spec scenario implemented alongside the code
      above.

## 3. BFS — Cross-surface regression and completeness

- [x] Existing CLI behaviour unchanged
      (`forge identity change-password` still reads from
      the terminal and uses the same Core function).
- [x] No registry/journal schema change; no Core
      migration; no CLI migration.
- [x] Full forbidden-sink + inline-script + path-leak +
      secret-leak sweep over the new surface (Rust +
      frontend). The new Rust handler never logs or
      echoes any password field; the new frontend block
      never logs or persists any password field.
- [x] No current-change placeholder/disabled behaviour
      remains; `IMPLEMENTED_WEB_ROUTES` agrees with all
      pins; old-frontend safety (a topbar without the
      new control still works — `Sign out` is the only
      topbar contract; the new control is additive).
- [x] New/touched Rust files stay under the source-file-
      size cap (`router.rs` ≤ 1000 verified after the
      new arm is added).

## 4. Verification (lightweight — manager runs the heavy pass)

- [x] `node scripts/check-openspec-change-names.mjs` PASS.
- [x] `git diff --check` clean.
- [x] `openspec validate forge-portal-identity-rotation
      --strict --no-interactive` 0 failures. The manager
      runs `openspec validate --all --strict --no-interactive`,
      `cargo build`, `cargo test --workspace`, and
      `forge gate`; this change is shipped with the
      lightweight checks above.
- [x] No push; two commits (proposal only, then
      implementation + tests).
