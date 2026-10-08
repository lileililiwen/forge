# Proposal: Real, deep-linkable dashboard navigation routes

## Why

The operator opened the dashboard and found the sidebar navigation is fake:
"All projects / Workbench / Manage projects / Portfolio / Delivery" are
fragment anchors (`href="#workbench"`, `#management`, `#portfolio`,
`#delivery`) on one long page. Clicking one only scrolls to a section and the
URL becomes `http://127.0.0.1:4173/index.html#management` — a label that jumps,
not a page. Every "page" is a `<section>` rendered at once on a single scroll,
so the operator cannot paste a link to a view, cannot use back/forward between
views, and sees every other section at the same time. The operator wants real
destinations: real, distinct, deep-linkable URLs, each showing only its own
view.

The static server (`src/web.rs`) is an exact-path allowlist; `/workbench` is a
404 today, so the routes must be taught to it as well.

## What Changes

- Add clean SPA path routes served as the one application shell: `/projects`,
  `/workbench`, `/management`, `/portfolio`, `/delivery` all return
  `index.html`. `/` keeps serving `login.html`; `/login.html`, `/index.html`,
  `/styles.css`, `/config.js`, `/app.js` are unchanged; every other path still
  404s. The allowlist stays exact-match with no traversal.
- Add `<base href="/">` to `frontend/index.html` so `styles.css`, `config.js`
  and `app.js` resolve from the root when the shell is served from a sub-path.
- Replace fragment navigation in `frontend/index.html` with real path links:
  All projects → `/projects`, Workbench → `/workbench`, Manage projects →
  `/management`, Portfolio → `/portfolio`, Delivery → `/delivery`.
- Add a small vanilla-JS router in `frontend/app.js`: it maps the current
  pathname to one view, shows only that view (`hidden` on the others), updates
  the nav active/`aria-current` state, the topbar breadcrumb and the document
  title, intercepts same-origin clicks on route anchors, and applies
  `history.pushState` / `popstate` so back/forward switch views.
- Make the fleet table's per-row action land on a real destination: a managed
  project's "Open" navigates to `/workbench`; an unmanaged project's "Manage"
  links to `/management`; the workspace hint link also targets `/management`.
  No `#` fragment remains as a navigation mechanism.
- New `tests/forge_web_navigation_contract.rs` starts `forge web serve` on an
  ephemeral port against a temp root and asserts each real route returns 200
  with the shell bytes, `/` still serves the login page, an unknown path and a
  traversal attempt still 404, and the sidebar nav carries no `href="#…"`
  navigation anchor. `src/web.rs`'s own `asset_name` unit test is updated.

## BFS Impact Map

- **Capabilities:** `forge-web-navigation-routing` (new).
- **Users / flows:** the operator pastes `http://127.0.0.1:4173/portfolio` and
  gets the portfolio view; each nav click is a real, shareable URL.
- **Contracts / data / persistence:** no JSON API, registry, journal or schema
  change. The only wire contract touched is the static server's URL allowlist.
- **Integrations / configuration:** none. `forge web serve` flags
  (`--bind`, `--port`, `--root`) and `scripts/web.sh`'s symlinking staging are
  unchanged; new files under `frontend/` are picked up automatically.
- **Callers:** `src/web.rs` (`asset_name`), `frontend/index.html` (nav, base),
  `frontend/app.js` (router, fleet links), `frontend/styles.css` if a style is
  needed to hide a view; the new contract test; `src/web.rs` unit test.
- **Failure / boundary behavior:** unknown path 404; traversal 404; `/` still
  login; a deep link to a route reloads to that view; browser back/forward
  switch views; a view whose data fails to load shows its existing error
  surface, not another view.
- **Tests:** `tests/forge_web_navigation_contract.rs` (new) and the updated
  `src/web.rs::static_server_has_an_allowlist_and_serves_login_at_root`.
- **Privacy / security:** the server remains an exact allowlist with no
  traversal and no arbitrary file reads; CSP/security posture
  (`X-Content-Type-Options`, `Cache-Control: no-store`) is unchanged; auth and
  session behavior are unchanged (`/` still login, session gate unchanged).

## Capabilities

- `forge-web-navigation-routing`: the dashboard's sidebar destinations are
  real, distinct, deep-linkable paths served as the app shell, each showing
  only its own view.

## Non-goals

- No client-side framework and no build step; vendored vanilla JS only.
- No separate HTML page per destination (see design §1 for why).
- No change to the JSON API, auth/session behavior, catalog, or static-server
  security headers.
- No per-project URL parameter (e.g. `/workbench?project=…`) in this change.
- No CLI path changes, so no `command_catalog` pinned-count change.
