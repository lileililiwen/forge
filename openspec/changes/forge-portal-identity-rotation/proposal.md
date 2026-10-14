# Proposal: forge-portal-identity-rotation

## Why

A Forge operator who rotates the workspace's administrator
password today has to drop to a terminal and run
`forge identity change-password` (`src/cli/commands_services.rs:45`,
`src/cli/identity.rs:57-78`). The portal carries the same Forge-wide
admin session every other admin route uses
(`src/api/admin/gateway.rs:512-644`), but exposes no
`Change password` control anywhere — the topbar's only session-bound
action is `Sign out` (`frontend/index.html:26`).
`README.md:156-159` explicitly tells the operator to use the CLI
for the rotation. That's a gap a sibling operator can close in one
click: the same session that signs them in can rotate the password
that minted it, with the same `change_password` Core call the CLI
runs (`src/identity/global.rs:172-204`) — same Argon2id hash
update, same `revoke_all_sessions` cascade, same email preserved.

## What Changes

- **One global session-gated admin POST** that wraps the existing
  `forge::identity::global::change_password` Core call:
  `POST /v1/admin/identity/change-password` — JSON request
  `{ "password": "...", "confirm": "...", "current": "..." }`,
  JSON response `{ "changed": true, "contract":
  "forge-admin-login/1.0.0" }` on `200`. Reuses the existing
  global `guarded` / `cors` gate (`src/api/admin/gateway.rs`) so a
  missing or expired `forge_admin_session` cookie is rejected as
  `401 api-unauthorized` *before* any Core call, and an
  unconfigured registry (no `forge_admin` row) returns
  `403 admin-identity-not-configured` rather than silently
  initializing. The handler lives beside the rest of the
  identity-shaped routes in a new
  `src/api/admin/identity.rs` (the read-only
  `src/api/admin/agent_identity.rs` precedent — `router.rs` is at
  999/1000 lines, so the handler is matched beside the table; see
  `src/api/admin/mod.rs:26-32`). The handler delegates to
  `forge::identity::global::change_password`; the Core function
  already enforces the 12–1024 character length, refuses an
  unconfigured registry, rotates the Argon2id hash, and calls
  `revoke_all_sessions_in` (every pre-rotation session is
  invalidated atomically with the hash update —
  `src/identity/global.rs:172-204`). No CLI change, no Core
  change, no schema change, no new dependency.
- **Catalog:** `identity.change-password` leaf converts from
  `cli_only` to `web_at` (one row in
  `src/api/command_catalog/rows_platform.rs`; the existing
  `REASON_TTY_HIDDEN` mapping on this row stays as a fallback for
  any future caller that is not the browser — the row's reason
  text is updated to keep the TTY/secret call-out honest). One
  new `ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD` constant in
  `src/api/admin/routes.rs`; one new `WEB_ROUTE_ADMIN_*` constant
  and one new `IMPLEMENTED_WEB_ROUTES` entry in
  `src/api/command_catalog/routes.rs`. The catalog pin count in
  `tests/forge_web_command_catalog_contract.rs` stays the same
  (conversion, not addition).
- **Frontend:** the dashboard topbar gains a `Change password`
  control next to the existing `Sign out` button
  (`frontend/index.html:24-27`). Clicking it opens a native
  `<dialog>` modal with three fields (`Current password`,
  `New password`, `Confirm new password`) and a confirmation
  checkbox reading "I confirm rotating the Forge administrator
  password and revoking every active browser session" (the
  workbench confirmation-tick pattern, e.g. `frontend/index.html:408`).
  Submit is disabled until all three fields are non-empty, the
  two new-password fields match, and the confirmation is ticked.
  The current password is verified client-side by re-using the
  existing `POST /v1/admin/session` shape with the supplied
  current password; a mismatch is reported inline as
  "Current password is incorrect" without re-rendering the whole
  form. On success the form posts to the new route, the response
  is rendered as a `role=status` notice ("Password updated —
  every other browser session has been revoked. Sign in again
  with your new password."), and the operator's own session is
  followed by an automatic sign-out + redirect to
  `login.html?next=...` because the Core call invalidates every
  token. The existing confirmation-tick + per-field-error +
  `renderErrorSummary` patterns (`frontend/app.js:3540-3560`,
  `:3495-3529`) are reused verbatim — no new dependency, no
  `innerHTML`, no inline script, no shell, no path interpolation
  in the JS.
- **Tests:** new
  `tests/web_portal_identity_rotation_contract.rs` pins: (1) the
  static token contract over the shipped `frontend/` assets —
  the dialog id is in `frontend/index.html`, the click handler
  is wired in `frontend/app.js`, no `innerHTML` / `eval(`
  / `Function(` in the new block, no POST other than the named
  route; (2) the in-process admin route contract through
  `forge::api::handle` — anonymous → `401`, missing content-type
  → `415`, weak password → typed `400` (matching the CLI's
  `change_password_refuses_weak_input_and_missing_administrator`
  boundary), mismatched confirmation → typed `400` (a UI-side
  guard surfaced as a typed refusal so the test is honest about
  what the server rejects), unconfigured registry → typed `403`,
  success → `200` with `changed: true` and a fresh
  `authenticate` round-trip works on the new password while the
  old one and every pre-rotation session is invalid. The CLI's
  `forge_web_login_credentials_contract::change_password_stdin_rotates_and_invalidates_a_prior_session`
  boundary is not duplicated — its assertions live at the CLI
  level; the new test mirrors the *server-side* surface.

## BFS Impact Map

- **Requirements/scenarios:** `control-plane-portal` (topbar UI),
  `central-admin-identity` (the change-password side of the
  Forge-wide admin session). No new canonical requirement; a
  small delta to `control-plane-portal` records the new
  topbar affordance. The 401/403/415 boundary tests already
  exist in `tests/forge_admin_api_contract.rs` and
  `tests/web_login_credentials_contract.rs`; the new contract
  test asserts the rotation-only behaviours (weak input refused,
  confirmation mismatch refused, unconfigured registry refused,
  rotate-then-rotation-revokes-sessions) without redoing the
  generic admin gate.
- **Concepts/modules:** `identity::global::change_password` (the
  single Core function the new route delegates to — unchanged);
  new `src/api/admin/identity.rs` handler + matcher; `Route`
  enum + `router.rs` threading (one new variant, one new arm in
  `route_request`, two in `required_permission`/dispatch
  — `router.rs` is at 999/1000, so the new route matcher and
  handler live beside the table in the new file, mirroring the
  `agent_identity.rs` precedent so the line-count cap holds);
  one constant in `src/api/admin/routes.rs`; catalog leaf
  conversion in `src/api/command_catalog/rows_platform.rs` +
  `src/api/command_catalog/routes.rs`; topbar control in
  `frontend/index.html`; dialog + click handler in
  `frontend/app.js`; new contract test.
- **Contracts:** `ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD` =
  `POST /v1/admin/identity/change-password`; one new `Route`
  variant; one new `WEB_ROUTE_ADMIN_*` constant; one new
  `IMPLEMENTED_WEB_ROUTES` entry; response carries
  `contract: "forge-admin-login/1.0.0"` (the same contract id
  the CLI emits in `src/cli/identity.rs:76`); `API_CONTRACT_VERSION`
  unchanged.
- **Callers/persistence:** no Core / CLI / registry-schema change.
  The handler opens the same registry `global::change_password`
  already opens; the new admin route is the only new caller.
- **Integrations:** no provider, no adapter subprocess, no git
  write, no native toolchain, no network on any path; the
  password is sent as a single JSON field over the existing
  CORS-gated admin session — the same posture the CLI uses
  (`scripts/web.sh reset-password` is the precedent for
  rotation in scripts, not in the browser).
- **Tests/compatibility:** new contract binary
  `tests/web_portal_identity_rotation_contract.rs`; regression
  on `tests/forge_admin_api_contract.rs` and
  `tests/forge_web_command_catalog_contract.rs`. Old-frontend
  safety: a topbar that does not contain the new control still
  works — `Sign out` is the only topbar contract; the new
  control is additive.
- **Concerns:** security (admin session cookie gate before any
  Core call; CORS-exact origin; password field never echoed;
  the new password is sent over the existing secure-channel
  path the sign-in form already uses; the operator's own
  session is invalidated by the same `revoke_all_sessions_in`
  call the CLI uses, so the post-rotation auto-sign-out is
  honest); quality (no placeholder, no new dep, no `innerHTML`,
  source-file-size caps); a11y (live regions, focus, native
  `<dialog>`, error-summary focus, `aria-describedby` /
  `aria-invalid` per field — exactly the login-form pattern at
  `frontend/login.html:36-44`).
- **Verification:** see `tasks.md` §4. The manager's
  "lightweight checks only" instruction is honoured: this
  change skips `cargo build`, `cargo test --workspace`,
  `forge gate`, and `openspec validate --all --strict` —
  the manager runs the heavy pass.

## Capabilities

- A Forge operator with a valid `forge_admin_session` can rotate
  the Forge-wide administrator password from the browser
  topbar, with the same Core rotation + session-revocation
  guarantees the `forge identity change-password` CLI already
  provides.
- A Forge operator who lacks the current password cannot rotate
  it (the browser reuses the sign-in credential check inline,
  with no extra server round-trip beyond the existing
  `POST /v1/admin/session` shape).

## Non-goals

- Initial administrator setup (`identity.setup`): the registry
  has no `forge_admin` row yet, so there is no admin session to
  gate on; setup remains a CLI-only first-run step
  (`scripts/web.sh start` is the entry point — README gap is
  documented, not closed here).
- Password recovery / reset: requires an out-of-band email
  channel that the Forge-wide admin plane deliberately does not
  ship; the existing `scripts/web.sh reset-password` script
  remains the recovery path.
- `identity.generate-password`: the browser already has no
  secure place to render a freshly generated password without
  persisting it; stays CLI-only (`REASON_TTY_HIDDEN`).
- `identity.status` (CLI read of `configured` / `email`): the
  login page already renders the equivalent honest state via
  `GET /v1/admin/session` (`frontend/app.js:347-353`).
- A "change email" flow: not in `forge identity` today and
  outside the audit gap.
- Showing the password in plaintext, autofilling the new
  password, or saving the password to `localStorage`: the
  browser mirrors the login form's `autocomplete="new-password"`
  + `type="password"` + show/hide toggle pattern, nothing more.
