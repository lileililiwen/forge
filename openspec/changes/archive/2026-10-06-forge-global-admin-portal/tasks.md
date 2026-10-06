# Tasks: forge-global-admin-portal

## BFS baseline

- [x] 1.1 Map API routing/auth, registry DB migrations, CLI identity dispatch, existing Rust-rendered portal behavior and contract fixtures.
- [x] 1.2 Record baseline format/build and focused identity/API/portal contract results; record local API process/port and available frontend preview tools.
- [x] 1.3 Finalize the frontend/API split contract: standalone `frontend/` static assets, configured frontend origin, JSON endpoints, cookie/CORS/Origin policy.
- [x] 1.4 Additive DB design, Argon2id dependency policy and lockfile/license/advisory review.

## DFS requirement implementation

- [x] 2.1 Implement interactive `forge identity setup --email` with hidden password confirmation, validation, duplicate refusal and Argon2id storage.
- [x] 2.2 Implement Forge-wide password verification and opaque expiring/revocable session lifecycle; persist token digests only.
- [x] 2.3 Add JSON `GET/POST/DELETE /v1/admin/session` and authenticated `GET /v1/admin/projects`; enforce configured-origin CORS and Origin checks without changing existing API/OIDC behavior.
- [x] 2.4 Build standalone `frontend/login.html`, `frontend/index.html`, `frontend/styles.css`, `frontend/config.js` and `frontend/app.js`; implement login, full fleet dashboard, search/filter and sign-out without project-id login or fake actions.
- [x] 2.5 Implement `forge web serve` in Rust with an allowlisted asset root, and document separate web/API startup commands and origin configuration for a repeatable local preview.

## BFS regression and completeness

- [x] 3.1 Verify setup success, duplicate/TTY/password boundaries, hash persistence, login success/failure, token expiry/revocation and cookie attributes.
- [x] 3.2 Verify admin API anonymous/valid/expired/revoked sessions, CORS origin allow/deny, logout origin checks, project OIDC isolation and existing `/v1` bearer compatibility.
- [x] 3.3 Verify frontend login/no-project-id, initialized/uninitialized states, complete populated/empty fleet, search/filter, honest error/stale states, keyboard navigation and 320px reflow.
- [x] 3.4 Review all impact-map callers, migrations, secret paths, project compatibility, frontend/API separation and Rust sources for newly embedded UI markup/styles/scripts.

## Verification

- [x] 4.1 Run required format/build, focused identity/API/portal/browser checks, OpenSpec name preflight, strict validation and `git diff --check`.
- [x] 4.2 Rebuild and run the API and static frontend separately on loopback; manually verify first-run instructions, bad login, successful global dashboard, project search and sign-out.
- [x] 4.3 Record exact commands/results, archive with promoted canonical specs only after all requirements pass, and update the HANDOFF pointer.
