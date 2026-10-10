# Design: Web command-reference browser

## 1. Ownership and placement

The reference is a section of the **projects view**, not a new view.
`src/web.rs::asset_name` stays an exact allowlist (`/projects`,
`/workbench`, `/management`, `/portfolio`, `/delivery`); adding a
route would churn the web listener, its unit test and the navigation
contract for zero data gain, since the catalog is already fetched on
dashboard boot. Placement: `<section id="command-reference">` inside
`#view-projects`, after the fleet project panel, so every signed-in
operator lands on it and it never interferes with the workbench,
management, portfolio or delivery views.

Owner: `frontend/` assets only. No Rust change of any kind.

## 2. Data contract (reused, not changed)

`loadCommands` already fetches `GET /v1/admin/commands` once into
`catalogCommands` (`{commands, categories, contract}`). The reference
renders from that same array; on fetch failure it renders the honest
"catalog unavailable" empty state (the existing `.catch` path sets
`catalogCommands = []`). No second fetch, no query params, no POST.

Row fields consumed: `id`, `label`, `summary`, `category`,
`availability`, `route` (web only), `cli_invocation`, `reason`
(non-web only). `execution` blocks are ignored — the reference never
executes.

## 3. Rendering

`renderCommandReference()` (called from both `loadCommands` arms and
from the filter-control listeners):

- Controls: `#cmdref-search` (type=search, labelled), `#cmdref-availability`
  (select: all six `AVAILABILITY_LABELS` values), `#cmdref-count`
  (`role="status"`, e.g. "Showing 234 of 234 commands"), `#cmdref-list`
  (the rows), `#cmdref-copy-status` (`role="status"`, copy feedback).
- Filter: case-insensitive substring over `id`, `label`, `summary`,
  `cli_invocation`, `reason`, `category`; availability select exact-matches
  the row state. Both combine (AND).
- Row (`article.cmdref-row`):
  - header: `<code>` id + availability badge (`makeBadge` with
    `AVAILABILITY_LABELS`/`AVAILABILITY_BADGE` — text always present,
    never color-only) + category label;
  - `<p>` summary (always; every catalog row carries one);
  - non-web: `<p>` reason (always; catalog invariant guarantees it) +
    CLI line (`<code>` exact `cli_invocation`) + `Copy` button;
  - web: `Open in <view>` anchor to the mapped existing view (real
    path, router-intercepted, hard-load safe) + CLI line as text (no
    copy needed; the action lives on its view — still show the string
    so the terminal equivalent is visible).
- Empty states: catalog empty → "Command reference unavailable…"
  (API/catalog unreachable); filter matches nothing → "No matching
  commands…" with a Clear hint. Both plain text, no controls that imply
  action.

## 4. Route→view mapping

`referenceViewForRoute(route)` maps the row's existing typed route to
the view that already serves it (checked order):

1. contains `delivery` → `/delivery`;
2. contains `portfolio` → `/portfolio`;
3. contains `workspace` → `/management`;
4. `POST /v1/admin/projects/new|import|register` → `/management`;
5. route is `GET /v1/admin/projects` or contains `fleet/status` →
   `/projects` (fleet lives there);
6. otherwise → `/workbench` (all remaining web routes are
   project-scoped: detail, plan/apply, status, maintain, classify,
   feature/spec, deploy/release/publish, graduation, intent,
   remediate, studio).

Unknown/empty route on a web row (cannot happen per catalog
invariants, but coded defensively) → `/projects`. Link labels name the
view ("Open in workbench"). Non-web rows never consult this mapping.

## 5. Failure boundaries

| Failure | Rendering |
|---|---|
| Catalog fetch fails / 401 → login | existing redirect; section shows "unavailable" state |
| `commands` missing / not an array | treated as empty → "unavailable" state |
| Row missing `reason` (non-web) | invariant violation; render "No reason was provided…" rather than omitting the line |
| Clipboard API absent / denied | "Copy unavailable — select the command above." + select-the-text fallback (existing pattern) |
| 234-row render cost | single fragment append; filter re-renders only |

## 6. Accessibility and style contracts

- Native `<input type="search">`, `<select>`, `<button>`, `<a>` —
  keyboard operable with visible focus (existing tokens).
- `#cmdref-count` and `#cmdref-copy-status` are `role="status"` live
  regions; error-summary focus is not applicable (no submitting form).
- 12px floor: row text, reason, CLI and count use `--text-md`
  (12px) or larger; only the availability badge keeps the compact
  `.badge` chip size, matching every existing badge.
- No motion added, so `prefers-reduced-motion` needs no new guard;
  no layout shift: controls reserve their row, list renders below.
- All catalog strings via `textContent`/existing `el()` helper —
  never `innerHTML` (the standing `innerHTML` contract failure is not
  extended).

## 7. Explicit non-changes

`src/web.rs`, router, catalog, API, CLI, registry, journal, sidebar
nav, `VIEW_BY_PATH`/`VIEW_IDS`/`VIEW_CRUMBS`, and every existing test
file are untouched. The change adds exactly: one HTML section, one
CSS block, one render/filter/copy function group, one test file.

## 8. Verification oracle

- `tests/web_command_reference_browser_contract.rs` (static tokens):
  section/controls ids; search+availability wiring; all six states
  filterable; badge reuse; reason + exact `cli_invocation` + copy on
  non-web rows with no executable control; web-row anchors to the five
  real paths; single shared catalog fetch (no new endpoint string);
  no `eval(`/`Function(`/`innerHTML` in the new code; clipboard +
  select fallback present.
- Manual oracle: throwaway API + web listeners, sign in, projects view
  shows the reference with all rows; search narrows; each availability
  filter shows only its state; every non-web row shows badge + reason
  + exact CLI + copy; every web row links to its view; copy works;
  zero JS console errors.
