# Design: forge-portal-identity-rotation

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  crate (library) plus `frontend/`. No sibling touched.
- **Modules changed:**
  - New `src/api/admin/identity.rs` — `route_identity` matcher
    plus one session-gated POST handler. Reuses
    `super::gateway::{guarded, cors, error}` (admin session
    gate + CORS-exact-origin + typed `error` envelope) and
    `forge::identity::global::change_password` (the single
    Core function the new route delegates to). Kept beside the
    table (not in `router.rs`) because `router.rs` is at
    999/1000 lines (`wc -l src/api/router.rs`) and the new
    route shape is a six-segment admin path that fits the
    `agent_identity.rs` precedent.
  - `src/api/admin/routes.rs` — one constant
    `ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD =
    "POST /v1/admin/identity/change-password"`. Kept beside
    the `ROUTE_ADMIN_IDENTITY_*` constants the agent_identity
    change introduced.
  - `src/api/model.rs` — one new `Route` variant
    `AdminIdentityChangePassword` (no `{id}`: the route is
    global, not project-bound; no `{session_id}`: not a
    session read). Doc comment records the route, the Core
    function it delegates to, and the no-CLI-change
    invariant.
  - `src/api/router.rs` — one arm in `route_request` matching
    `("POST", ["v1", "admin", "identity", "change-password"])`,
    placed before the generic four-segment admin `OPTIONS`
    wildcards so the literal `identity` segment never reads
    as a project id. One arm in `required_permission` returning
    `None` (the `guarded` gate does the real check; no
    additional `admin:access` permission is required beyond
    any valid session, matching the existing sign-in / sign-out
    posture). One arm in the `admin::handle` short-circuit list.
    The `not_found` exhaustiveness arm at the bottom of `handle`
    gains one variant. `router.rs` stays ≤ 1000 lines after
    the change.
  - `src/api/admin/mod.rs` — `pub mod identity;` declaration
    and a chain in `route_beside` (placed before `creation`
    so the new global `identity` literal never reads as a
    project id under `["v1", "admin", "projects", id, ...]`).
  - `src/api/admin/deploy.rs` — one new dispatch arm routing
    `Route::AdminIdentityChangePassword` to the new handler,
    following the same `guarded` + `cors` shape the
    agent_identity reads use.
  - `src/api/command_catalog/routes.rs` — one new
    `WEB_ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD` constant and
    one new `IMPLEMENTED_WEB_ROUTES` entry.
  - `src/api/command_catalog/rows_platform.rs` — the
    `identity.change-password` leaf converts from
    `not_yet_web`-equivalent (`cli_only`-shape) to
    `web_at(ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD)`; the row's
    description keeps the explicit TTY/secret call-out as a
    fallback reason for any future non-browser caller.
  - `frontend/index.html` — one topbar button next to
    `Sign out` (`#sign-out` at line 26) opening a
    `<dialog>` modal with three fields, one confirmation
    checkbox, and a `role=status` result region.
  - `frontend/app.js` — the click handler binds the
    topbar button to a single function that opens the
    dialog, validates inline, posts to the named route,
    and on success auto-signs-out (the Core call revoked
    every session including the operator's own) and
    redirects to `login.html?next=...`. Reuses
    `request`, `setFieldError`, `clearFieldError`,
    `renderErrorSummary`, `clearErrorSummary`,
    `setResultRole` (the login-form pattern at
    `app.js:359-405`).
  - `tests/web_portal_identity_rotation_contract.rs` —
    new contract binary (§5 oracle).

## 2. Ownership and contracts

- **Owner of truth:** Core. `forge::identity::global::change_password`
  owns the Argon2id hash, the 12–1024 character length
  check, the "no `forge_admin` row → refuse" check, the
  hash rotation, and the `revoke_all_sessions_in` cascade
  (`src/identity/global.rs:172-218`). The handler does not
  duplicate any of those checks; it forwards the typed
  errors Core returns to the JSON envelope without
  reinterpretation.
- **Request shape:**
  ```json
  {
    "current": "current-password-as-typed-by-the-operator",
    "password": "new-password-at-least-12-characters",
    "confirm":  "new-password-must-equal-password"
  }
  ```
  All three fields are non-empty strings. `password` and
  `confirm` are required by Core; `current` is required by
  the handler so the operator cannot rotate the password
  from a borrowed session (a "did you leave yourself logged
  in?" guard) — verified by re-using the sign-in
  credential check inline, *not* by inventing a new
  `forge::identity::global::verify` Core call.
- **Response shape on `200`:**
  ```json
  {
    "changed": true,
    "contract": "forge-admin-login/1.0.0"
  }
  ```
  Contract id is the same one the CLI emits in
  `src/cli/identity.rs:76` and `src/cli/identity.rs:54`,
  so the browser and the CLI agree on the same value
  without a new constant.
- **Response shape on refusal:**
  - `400 admin-content-type-required` — request is not
    `application/json`. Mapped from the existing
    `gateway::sign_in` 415 pattern.
  - `400 admin-invalid-request` — body is not valid JSON
    or is missing a required field. Never echoes the
    offending input.
  - `400 admin-password-too-weak` — `password` length is
    outside 12–1024 characters. Typed refusal (Core's
    `Err("password must contain 12 to 1024 characters")`
    string is matched once and mapped; the
    `admin-password-too-weak` code is the wire shape, the
    Core string is the source of truth).
  - `400 admin-password-confirmation-mismatch` —
    `password` ≠ `confirm`. Mapped in the handler, not
    in Core, because Core's `change_password` does not
    see a confirmation field.
  - `401 api-unauthorized` — missing or expired
    `forge_admin_session` cookie. Mapped by the `guarded`
    gate before any Core call.
  - `403 admin-identity-not-configured` — registry has
    no `forge_admin` row. Mapped from Core's
    `Err("no Forge administrator is configured; ...")`
    string; same mapping principle as the 400 case.
  - `403 admin-current-password-incorrect` —
    `current` does not match the stored hash. Mapped
    inline by the handler (see §3 for the verification
    shape; the wire code is `admin-current-password-incorrect`,
    not `api-unauthorized`, so the operator knows the
    password itself is wrong rather than that the
    session ended).
  - `413 admin-request-too-large` — body > 8 KiB. Same
    gate as `sign_in`.
  - `415 admin-content-type-required` — content-type is
    not `application/json`. Same gate as `sign_in`.
- **Errors never echo the password:** the typed refusal
  envelope uses the standard `{ "error": { "code", "message" } }`
  shape; the `message` is one of the static strings above
  and never the input value. The `current`, `password` and
  `confirm` fields are *not* projected into the response
  on any code path.

## 3. Frontend contract

- **Topbar:** the existing `#sign-out` button is unchanged.
  A new `<button id="identity-change-password"
  class="button button-quiet" type="button">Change password</button>`
  sits to its left in the same `<header class="topbar">`
  (the `flex` order is left-to-right: workspace crumb,
  Change password, Sign out). The button is keyboard-operable
  (native `<button>`), screen-reader labelled (visible text),
  and `aria-haspopup="dialog"` so assistive tech announces
  the modal.
- **Dialog:** a `<dialog id="identity-change-password-dialog">`
  element with one `<form>` (native submit, no JS-driven
  form), three `<input type="password">` fields, one
  confirmation checkbox, and one submit button. The
  `autocomplete` attributes are `current-password`,
  `new-password`, `new-password` respectively (the
  password-manager hints the login form already uses).
  A show/hide toggle on the new-password fields mirrors
  the login form's `#login-password-toggle`. The submit
  button is disabled until the three fields are non-empty,
  the two new-password fields match, and the confirmation
  is ticked — exactly the workbench confirmation-tick
  pattern (`frontend/index.html:408-413`).
- **Submit path:** `app.js` calls `request("/v1/admin/identity/change-password", { method: "POST", headers: { "Content-Type": "application/json", Accept: "application/json" }, body: JSON.stringify({ current, password, confirm }) })`.
  On `200` the operator's own session is invalidated by
  Core's `revoke_all_sessions_in` (the same call the CLI
  uses), so the page then calls
  `request("/v1/admin/session", { method: "DELETE" })` and
  `window.location.replace("login.html?next=" + encodeURIComponent(currentPath))` —
  the existing post-sign-out redirect shape.
- **Failure path:** a `400 admin-password-too-weak` /
  `admin-password-confirmation-mismatch` /
  `admin-current-password-incorrect` response is rendered
  into the dialog's per-field error region via
  `setFieldError` + `renderErrorSummary`; the focus moves
  to the summary. A `401 api-unauthorized` triggers the
  existing `sessionExpiredRedirect` (the operator's
  session died between open and submit — same as
  `login.html`). A `5xx` / network failure shows a single
  inline error in the dialog result region.
- **Forbidden sinks in the new block:** no `innerHTML`,
  no `eval(`, no `Function(`, no inline script, no
  `XMLHttpRequest`, no shell — verified by the contract
  test.

## 4. Catalog wiring

- `identity.change-password` →
  `web_at(ROUTE_ADMIN_IDENTITY_CHANGE_PASSWORD)`. Row
  reason keeps the TTY/secret call-out as a fallback
  for any future non-browser caller. The
  `REASON_TTY_HIDDEN` mapping on the existing row is
  updated to mention the browser as the supported caller
  while keeping the secret-without-echo posture.
- `IMPLEMENTED_WEB_ROUTES` +1. The catalog count pin
  in `tests/forge_web_command_catalog_contract.rs`
  stays the same (conversion, not addition).
- `identity.setup` stays `not_yet_web` (no admin session
  to gate on at first run).
- `identity.generate-password` stays `not_yet_web` (no
  safe browser rendering path; `REASON_TTY_HIDDEN`).
- `identity.status` stays `cli_only` (login page
  already renders the equivalent).

## 5. Failure boundaries

- Anonymous → `401 api-unauthorized` (guarded gate)
  *before* any Core call.
- Wrong content-type → `415 admin-content-type-required`.
- Body > 8 KiB → `413 admin-request-too-large`.
- Body not JSON or missing a required field → `400
  admin-invalid-request` without echo.
- `password` length outside 12–1024 → `400
  admin-password-too-weak` (Core's typed string is
  the source of truth).
- `password` ≠ `confirm` → `400
  admin-password-confirmation-mismatch`.
- `current` does not authenticate → `403
  admin-current-password-incorrect`.
- Registry has no `forge_admin` row → `403
  admin-identity-not-configured`.
- Refusal under any code path: the `forge_admin` hash
  is not updated, no session is revoked, no journal
  row is written. Asserted by the contract test via
  before/after hash and session-count comparison.
- A successful rotation: the `forge_admin` hash is
  updated, every pre-rotation session is revoked, no
  journal row is written (the CLI path also writes
  none — the change is a Core-internal write). The
  contract test asserts the new password authenticates
  and the pre-rotation session is invalidated.

## 6. Why no CLI / Core change

- The CLI is the source of truth for the operator
  contract: rotating the password reads the new
  password from the terminal without echo
  (`src/cli/identity.rs:543-584`), confirms it against
  a typed second read, and forwards it to
  `forge::identity::global::change_password`. The
  browser path is the same Core function with a JSON
  envelope; the terminal echo / confirmation posture
  is a CLI concern and stays a CLI concern.
- Core is the source of truth for the storage
  contract: the Argon2id hash, the 12–1024 character
  length, the unconfigured-registry refusal, the hash
  update, and the `revoke_all_sessions_in` cascade
  are already exactly what the browser needs. Adding
  a parallel Core call (e.g. a `change_password_v2`
  that returns structured errors instead of `String`)
  would duplicate the storage contract for no
  observable benefit; the JSON mapping lives in the
  admin handler where the wire shape belongs.
- The catalog reason update (§4) is the only piece
  of metadata the CLI catalogue cares about; the
  CLI binary itself is untouched.

## 7. Test oracle (contract binary)

- **Static asset contract (no server):**
  - `frontend/index.html` contains
    `id="identity-change-password"`,
    `id="identity-change-password-dialog"`,
    `id="identity-change-password-current"`,
    `id="identity-change-password-new"`,
    `id="identity-change-password-confirm"`,
    `id="identity-change-password-confirm-tick"`,
    `id="identity-change-password-submit"`,
    `id="identity-change-password-result"`,
    `id="identity-change-password-error-summary"`.
  - The new block in `frontend/app.js` does not
    contain `innerHTML`, `eval(`, `Function(`,
    `XMLHttpRequest`, or `.command`.
  - The new block posts only to
    `/v1/admin/identity/change-password` and to
    `/v1/admin/session` (the post-rotation
    sign-out), no other admin route.
- **Live in-process admin route contract (through
  `forge::api::handle`):**
  - Anonymous `POST /v1/admin/identity/change-password`
    → `401 api-unauthorized` (the existing admin gate
    is reused; the `tests/forge_admin_api_contract.rs`
    coverage of the gate is not duplicated).
  - Missing `content-type: application/json` →
    `415 admin-content-type-required`.
  - Body > 8 KiB → `413 admin-request-too-large`.
  - `password` < 12 characters with a valid session
    and a configured registry → `400
    admin-password-too-weak`; the registry hash is
    *unchanged* and the pre-rotation session is
    *still valid* (mutation-guard).
  - `password` ≠ `confirm` → `400
    admin-password-confirmation-mismatch`; the
    registry hash is *unchanged* and the pre-rotation
    session is *still valid* (mutation-guard).
  - `current` does not match the stored hash → `403
    admin-current-password-incorrect`; the registry
    hash is *unchanged* and the pre-rotation session
    is *still valid* (mutation-guard).
  - Unconfigured registry → `403
    admin-identity-not-configured`; no row is
    inserted.
  - Success: `200 { changed: true, contract:
    "forge-admin-login/1.0.0" }`; the new password
    authenticates; the pre-rotation session is
    invalid; the email is preserved.
- **Catalog pin:** the command catalog at
  `/v1/admin/commands` reports
  `identity.change-password` with `availability:
  "web"` and `route: "POST
  /v1/admin/identity/change-password"` (the same
  pin the agent_identity contract test uses for
  the five reads).
- **Forbidden duplicate of existing boundaries:**
  the 401/415/413 cases are asserted once each
  (the `forge_admin_api_contract.rs` suite already
  pins the generic admin gate; the new test
  asserts only that the new route honours the
  same gate — the failure codes are
  byte-identical).
