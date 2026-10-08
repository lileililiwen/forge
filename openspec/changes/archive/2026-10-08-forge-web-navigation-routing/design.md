# Design: Real, deep-linkable dashboard navigation routes

## 1. Routing decision: SPA path routing over separate HTML pages

Two shapes were considered for a real destination per sidebar item:

- **(a) SPA path routing (chosen).** `forge web serve` maps `/projects`,
  `/workbench`, `/management`, `/portfolio`, `/delivery` to the single
  `index.html` shell; `index.html` gains `<base href="/">`; `app.js` selects
  the view from `location.pathname` and switches views with `history.pushState`
  + `popstate`.
- **(b) Separate real HTML page per destination.** Five shell copies, each
  embedding the same sidebar, topbar and scripts, each mapped to its own path.

(a) is chosen because this server is a strict exact-path allowlist and the
browser app already runs one `dashboardPage()` that loads and wires every
section. (b) would duplicate the shared shell markup five times — a sidebar or
topbar edit would need five edits — and would need five allowlist entries and
five near-identical files to stay in sync, for zero behavioural gain: the
sections are already all in one document and switching is just visibility. (a)
keeps one shell, one allowlist addition of four paths (plus `/projects`), and
one view-selection function; it also makes assets resolve correctly from a
sub-path via `<base href="/">`, which (b) would need per file too.

Real anchors are kept (`href="/workbench"`, not JS-only buttons), so navigation
still works as a real same-origin request that lands on the right view if JS
has not yet run or is disabled; the router only upgrades the click to a
`pushState` view swap.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library + binary) and the vendored `frontend/`. No sibling touched.
- **Files changed:**
  - `src/web.rs` — `asset_name` maps the five dashboard paths to
    `index.html`; the unit test is extended.
  - `frontend/index.html` — `<base href="/">`; real nav `href`s; view wrapper
    `#view-projects`; a `#topbar-crumb` target; `hidden` on the non-default
    view sections; the mismatched Data-sources `</section>` is balanced.
  - `frontend/app.js` — `VIEW_BY_PATH` / `viewForPath` / `renderRoute` /
    `navigateTo`, a document click interceptor, a `popstate` listener; the
    two `#management` links become `/management`; `openInWorkbench` navigates
    to `/workbench`.
  - `frontend/styles.css` — only if a `[hidden]`/view rule is needed (the
    existing global styles already hide `hidden` elements).
  - `tests/forge_web_navigation_contract.rs` — new oracle.
- **Modules reused unchanged:** the rest of `serve` / `serve_one` /
  `write_response`, the JSON API, the catalog and `scripts/web.sh`.
- **Must NOT change:** the API contract, auth/session behavior, the server's
  security headers, any other CLI path, or `command_catalog` (`forge web
  serve` already exists; no new Clap path is added).

## 3. Server contract (`src/web.rs`)

`asset_name` stays a pure exact-match allowlist. New arms:

```text
"/" | "/login.html"                                  => "login.html"
"/index.html" | "/projects" | "/workbench"
  | "/management" | "/portfolio" | "/delivery"       => "index.html"
"/styles.css" | "/config.js" | "/app.js"             => themselves
_                                                    => None (404)
```

No path is joined, normalized or resolved from the request; the request path is
matched literally, so `..`, `%2e%2e`, query strings and any other path can
never select a file. `content_type("index.html")` is already `text/html`.

## 4. Frontend contract (`frontend/index.html`, `frontend/app.js`)

Path → view (default `projects` for `/`, `/index.html`, `/projects`):

| Path | View id | Nav label | Document title |
|---|---|---|---|
| `/`, `/index.html`, `/projects` | `view-projects` | All projects | Projects · Forge |
| `/workbench` | `workbench` | Workbench | Workbench · Forge |
| `/management` | `management` | Manage projects | Manage projects · Forge |
| `/portfolio` | `portfolio` | Portfolio | Portfolio · Forge |
| `/delivery` | `delivery` | Delivery | Delivery · Forge |

`renderRoute()`:

1. `view = viewForPath(window.location.pathname)`.
2. For each view element, `hidden = (id !== view)`; only the active view shows.
3. Toggle `nav-active` / `aria-current="page"` on the sidebar link whose
   `data-route` equals the view.
4. Set `#topbar-crumb` text and `document.title`.

`navigateTo(path)`: `history.pushState(null, "", path)` then `renderRoute()`.
A delegated `click` listener intercepts unmodified left-clicks on same-origin
anchors whose path is a known route, calls `preventDefault()` + `navigateTo`,
and otherwise lets the browser navigate. `window.addEventListener("popstate",
renderRoute)` makes back/forward switch views. `renderRoute()` runs once at
dashboard boot (before the async data load) so a deep link shows the right view
with no flash of the wrong one.

Anchors keep real `href`s, so a same-origin hard navigation to any route also
lands on the right view after the server returns the shell.

## 5. Failure and boundary policy

| Case | Behavior |
|---|---|
| `GET /workbench` (or any dashboard path) | 200 `text/html`, byte-identical `index.html`; router shows only that view |
| `GET /` | 200 `text/html`, `login.html` (unchanged) |
| `GET /does-not-exist` | 404 `text/plain` (allowlist miss) |
| `GET /../src/main.rs`, `/../../etc/passwd` | 404; the request path is never resolved or read |
| Deep link pasted into a fresh tab | server returns shell → router shows the matching view |
| Browser Back / Forward | `popstate` re-runs `renderRoute`; view, nav state and title follow the URL |
| Managed project row "Open" | navigates to `/workbench` (real path), then loads that project detail |
| Unmanaged project row "Manage" | anchors to `/management` (real path), no fragment |
| Workspace not-onboarded hint | anchors to `/management` (real path), no fragment |
| View's data fails to load | the section's existing error/notice surface renders; a failed section never reveals another view |
| Unknown or malformed route path on load | falls back to the `projects` view; no crash |

## 6. Compatibility

- `/`, `/login.html`, `/index.html`, `/styles.css`, `/config.js`, `/app.js`
  keep their exact current responses. Only additive routes appear.
- Login, session gate, sign-out redirect (`login.html`) and the JSON API are
  untouched.
- `scripts/web.sh` stages `frontend/` by symlinking every file; new frontend
  files need no script change.
- No CLI path is added, so `command_catalog` row count is unchanged.

## 7. Verification oracle

`tests/forge_web_navigation_contract.rs`, in the
`forge_web_maintainer_surface_contract.rs` / `forge_web_workspace_onboarding_browser.rs`
house style (`forge_bin()`, `free_port()`, `copy_dir()`, `ChildGuard`,
`wait_for_port`), with a raw `TcpStream` request helper (`api_contract.rs`
style):

1. Copy `frontend/` into a temp root; run `forge web serve --bind 127.0.0.1
   --port <ephemeral> --root <tmp>`.
2. Each of `/projects`, `/workbench`, `/management`, `/portfolio`,
   `/delivery`, `/index.html` returns 200 with `text/html` and bytes equal to
   the shell `index.html`.
3. `/` returns 200 with the `login.html` bytes.
4. `/does-not-exist` returns 404.
5. A raw `GET /../src/main.rs` (and `GET /../../etc/passwd`) returns 404 and
   never the file.
6. The sidebar `<nav>…</nav>` block in `frontend/index.html` contains no
   `href="#…"`; it contains the five real paths. `frontend/app.js` uses
   `history.pushState` and a `popstate` listener and sets no `href = "#…"`
   navigation link.

Existing suites that must stay green: `cargo test --lib command_catalog`,
`cargo test --bin forge`, `plugins_contract`, `catalog_contract`,
`classify_derive_contract`, `classify_apply_contract`,
`web_login_credentials_contract`, `forge_web_command_catalog_contract`,
`forge_web_maintainer_surface_contract`.

## 8. Decision ledger

- **Resolved:** SPA path routing (a) over separate pages (b); one shell, one
  router, additive allowlist.
- **Resolved:** real `<a href>` links plus click interception, so navigation is
  a real URL even before JS runs.
- **Resolved:** `<base href="/">` on `index.html` only; single-segment routes
  make relative assets resolve correctly and the base makes it explicit.
- **Resolved:** `/projects` is added as the explicit "All projects" route while
  `/` and `/index.html` still map to the same view for compatibility.
- **Resolved:** no per-project URL parameter in this change; the workbench view
  remains project-selected in-page.
- **Blockers:** none.
