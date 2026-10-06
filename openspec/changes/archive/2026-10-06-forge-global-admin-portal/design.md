# Design: forge-global-admin-portal

## Implementation boundary

Implement in `/home/paul/code/forge` only. `frontend/` owns all browser HTML, CSS and JavaScript and is served by a separate Rust static web listener on its own port. The API listener owns identity, sessions, registry access and JSON endpoints only. Do not add page markup, inline styles or browser scripts to Rust source or place browser pages beneath an API route/module. Keep the existing `src/api/ui` implementation untouched until frontend coverage is ready; this change must not make the existing portal less usable during the transition.

Extend `src/identity` for Forge administrator credential/session persistence and password verification; extend the existing registry database additively; wire `forge identity setup`, `forge api serve` and `forge web serve` in `src/main.rs`; add JSON endpoints to the API router and fleet serialization to the API data layer. Implement `src/web.rs` as a separate static web server with an allowlisted asset root. Add standalone `frontend/login.html`, `frontend/index.html`, `frontend/styles.css`, `frontend/config.js` and `frontend/app.js`. Add `frontend/README.md` with separate API and Rust web-server commands for manually opening the frontend and configuring its API origin. Add `argon2` as the password-hashing dependency; no external runtime service is required.

Do not change the AllTools repository, project manifest identity schema, project-scoped OIDC implementation, existing `/v1` bearer authorization contracts, or project mutation semantics.

## Language, runtime and local preview

Rust 2021, MSRV 1.87, blocking in-process HTTP/1.1 API transport, separate minimal Rust static HTTP server, SQLite and browser-native HTML/CSS/JavaScript. Use the locally available `argon2` crate (Argon2id) and existing `rand`, `sha2`, `rusqlite`, `chrono` and `libc` dependencies. The frontend loads from the independent Rust web server, with an API base URL supplied by `frontend/config.js`. Document and verify local preview commands for the API and web listeners, using loopback origins such as `http://127.0.0.1:8765` and `http://127.0.0.1:4173`.

The API must explicitly allow the configured frontend origin for credentialed CORS requests; it must never combine wildcard origins with credentials. Session cookies remain host-only and work across ports on the same loopback hostname. The frontend sends `credentials: "include"`; state-changing requests require JSON and validate the configured frontend `Origin` server-side. Avoid third-party frontend frameworks and build steps for this change.

## Ownership and shared code

Forge owns the global operator account, sessions, project data API, static web server and frontend source. Persist the global administrator in the Forge registry database because that is the database path already supplied to every API/CLI transport; project records remain isolated rows and do not own the Forge session. The web listener serves only the fixed frontend asset allowlist and does not proxy or implement API requests. AllTools supplies visual guidance only. Its auth storage and framework are incompatible with Forge's runtime and must not be imported or copied as implementation code.

## Behavioral model

| State | Event | Result |
|---|---|---|
| No Forge admin configured | `forge identity setup --email ...` with two matching hidden password entries | Validate email and password policy; hash with Argon2id; atomically create the one admin record. Refuse if already configured. |
| No Forge admin configured | Frontend loads and requests `GET /v1/admin/session` | Return an unauthenticated JSON state with setup instructions; do not return fleet data. |
| Admin configured, anonymous | Frontend submits email/password to `POST /v1/admin/session` | Verify credentials; on success mint an opaque expiring token and set the Forge session cookie; on failure return the same generic error for all credential failures. |
| Valid Forge session | Frontend requests `GET /v1/admin/projects` | Return all registered projects and real summary/evidence data, regardless of each project's identity configuration. |
| Valid Forge session | Frontend sends `DELETE /v1/admin/session` | Revoke only the current Forge session and expire its cookie. |
| Missing, invalid, expired or revoked session | Any protected admin API request | Return `401` JSON, clear stale cookie when applicable, and return no project data. |

Store an Argon2id PHC string, normalized email and account creation timestamp; store session token hashes, created/expiry timestamps and revocation state. Generate 256-bit tokens from the OS CSPRNG, send only the raw token in an `HttpOnly; SameSite=Lax; Path=/` cookie, and persist only its SHA-256 digest. Set `Secure` outside loopback development. Bound session TTL to 12 hours and enforce it on every protected admin API request. Cookie name is `forge_admin_session`, distinct from project OIDC `forge_session`.

The setup command requires an interactive terminal for hidden password input and confirmation, minimum 12 characters, and an email-shaped account identifier. It prints no secret. It is idempotent only as a refusal: a second setup does not replace the account. Password resets are outside this change.

## Frontend contract

- `frontend/login.html` contains the email/password form and a visual split-panel composition inspired by the supplied AllTools page. It must not ask for a project ID, show nonfunctional social-login controls, or put credentials in a URL. `frontend/config.js` supplies the API origin without embedding scripts in HTML.
- `frontend/index.html` contains the authenticated Forge-wide dashboard shell. `frontend/styles.css` owns all visual styles. `frontend/app.js` owns API calls, state transitions, project search/filter and sign-out behavior. No HTML strings are generated in Rust.
- `forge web serve --root frontend` serves the browser pages from an independent Rust web listener; `forge api serve` remains the JSON backend.
- On startup, the frontend calls `GET /v1/admin/session`; an anonymous response presents the login page, a valid session loads the dashboard, and an uninitialized account shows the exact local CLI setup command.
- The dashboard shows all projects, real registry status and available operation evidence, with honest empty, unavailable and stale states. Project IDs may be displayed in authenticated project rows/links only.
- Add responsive layout, keyboard operation, semantic landmarks, visible focus, reduced-motion support, and legibility at 320 CSS pixels.

## API contract and compatibility

- `GET /v1/admin/session`: returns JSON session state and setup state; never returns credentials or fleet data.
- `POST /v1/admin/session`: accepts bounded JSON `{ "email": ..., "password": ... }`; on success sets the session cookie and returns authenticated state; invalid credentials return a generic JSON `401` and do not create a session.
- `DELETE /v1/admin/session`: requires a valid session and exact configured frontend `Origin`; revokes the session and expires the cookie.
- `GET /v1/admin/projects`: requires a valid Forge admin cookie and returns all registered project rows plus real summary data.
- `OPTIONS` and CORS responses are limited to the configured frontend origin and the required methods/headers; wildcard credentialed CORS is forbidden. The default local origin is `http://127.0.0.1:4173`, overrideable with `FORGE_FRONTEND_ORIGIN`.
- No browser HTML route is added to the API. Existing `/ui` behavior remains available during this change; the new independent frontend is the Forge-wide login/dashboard entry to preview.
- New DB schema is additive and existing registry rows survive migration. Existing project OIDC cookie and API bearer behavior stay unchanged.
- CLI setup: `forge identity setup --email <address>`; password is read twice without terminal echo; the command refuses non-interactive stdin and duplicate initialization.

## Failure and boundary policy

Invalid email/password setup, mismatched confirmation, weak password, non-TTY setup, duplicate account, corrupt DB state or hash failure returns typed CLI failure and writes no partial admin. Invalid credentials have one generic response and create no session. Malformed/oversized JSON is refused. An absent/disallowed Origin cannot perform state-changing admin requests. Expired or unknown cookies authorize nothing. Passwords and raw tokens are not written to DB, logs, operation journal, browser storage, HTML or URL. Database errors fail closed and do not return fleet data.

## Verification oracle

- Identity contract: setup success, duplicate refusal, mismatched/weak password refusal, password hash is not plaintext, correct/incorrect verification, token raw/hash separation, expiry and revocation.
- API contract: login success sets only the Forge cookie; invalid credentials do not; anonymous/valid/expired/revoked sessions; full fleet is visible regardless of project `identity:` blocks; project OIDC cookie and `/v1` bearer compatibility; CORS origin allow/deny; logout origin mismatch does not revoke.
- Frontend contract: source files are under `frontend/`; no new markup/style/script is embedded in Rust; login has no project-ID control; dashboard shows all projects, real values, search/filter and honest empty/error states; sign-out returns to login; keyboard and 320px layout work.
- Run `cargo fmt --check`, `cargo build`, focused identity/API/UI contracts and OpenSpec checks. Run the rebuilt API and separate static frontend server, then manually verify first-run state, bad login, successful global dashboard, project search and sign-out in the browser. Record both local URLs and exact commands.
- Do not check off tasks or archive until these results exist.

## Decision ledger

- **Resolved:** Forge-owned local password account because the requested UI is email/password and Forge must authenticate independently of project identity. Account is initialized once by a local interactive CLI command.
- **Resolved:** browser files live in a standalone `frontend/` directory and are served by a Rust static web command; the separate Rust API returns JSON and owns identity/data only.
- **Resolved:** one change contains auth plus dashboard because they share one Forge-wide session and one end-to-end operator acceptance flow; a split leaves an unusable intermediate state.
- **Deferred:** multi-user roles, account recovery, provider federation, remote account lifecycle and replacing/migrating all legacy project detail HTML.
- **Blockers:** none.
