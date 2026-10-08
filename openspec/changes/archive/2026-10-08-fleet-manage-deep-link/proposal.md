# Proposal: Fleet per-project Manage entries land on the managing view

## Why

Opening `http://127.0.0.1:4173/management?project=alethefy` does not
focus/manage alethefy. Alethefy's actual live state (read from the running
pair without changing it):

- **Not registered.** The live registry holds 7 records (`argoscope`,
  `demoapp`, `localonly`, `mnemora`, `native-react-preview`, `net-app`,
  `portdemo42170`); none is alethefy and none points at
  `/home/paul/code/alethefy`.
- **Observed, unmanaged.** `GET /v1/admin/projects` reports alethefy as
  `management: "observed"`, `source: "published"` (forge-publish-history,
  25 such rows), `capabilities: []`, no conflict.
- **Onboardable.** The workspace sibling exists with no `forge.yaml`,
  import-decidable (`python-service`, high confidence), derived id
  `alethefy`; its workspace candidate is `unregistered` + selectable.
- **Fleet entry is correct.** Its row renders
  `<a href="/management?project=alethefy">Manage</a>` — the right
  destination for an unmanaged project.

Browser reproduction against a throwaway mirror of live (registry copy,
same project root, throwaway ports) shows two real defects behind the
report:

1. **Auth round-trip drops the deep link (the reported symptom).** Opening
   the exact URL with no session bounces to `login.html`; after sign-in the
   browser lands on `index.html` (projects view). `?project=alethefy` is
   lost and alethefy is never focused. An authenticated open ticks alethefy
   correctly, so the parameter handling works but the login handoff does not
   carry it.
2. **Workbench has no `?project=` support (the managed half).** A managed
   project's fleet action is an "Open" button that navigates to bare
   `/workbench` with no project in the URL — not shareable, lost on
   reload, invisible to back/forward. `/workbench?project=<id>` boots an
   empty workbench. And a manual `/management?project=<managed-id>` ends in
   a "cannot be onboarded (registered)" notice instead of the workbench
   that can actually manage that project.

## What Changes

- The workbench reads `?project=` on every route render (boot, reload,
  back/forward) and once the fleet has populated its project selector:
  a managed id selects and loads that project; an unknown id loads
  nothing and says so; an empty/absent id changes nothing.
- The fleet "Open" action for managed/self rows navigates to
  `/workbench?project=<identity>` (still a button, now deep-linkable).
- A `/management?project=<id>` whose workspace candidate is already
  `registered` redirects to `/workbench?project=<id>` instead of the
  cannot-onboard notice. All other non-selectable states (id-collision,
  undecidable, ambiguous, unknown id) keep the honest select-nothing
  notice.
- The login handoff carries the deep link: an unauthenticated dashboard
  load redirects to `login.html?next=<path+query>`; after sign-in (or when
  already signed in) the browser returns to `next` when it is a validated
  same-origin dashboard route, else `index.html`.
- Regression coverage for both halves: workbench deep-link browser
  oracle (managed boot, reload, unknown id, back/forward, registered-id
  redirect, login round-trip) plus static token assertions in the
  navigation contract test.

## Package Boundary and Split Assessment

Single outcome: **a per-project fleet action URL boots the view that can
manage that project, surviving reload, back/forward and the login
round-trip.** Included: `frontend/app.js` router/param/login-handoff
behavior and browser + static regression tests. Excluded: JSON API,
registry/journal, catalog, CLI, static-server allowlist (already serves
every query string as the shell), styles, and any publish/doctor
performance work (the known 27 s workbench health cost is recorded in
HANDOFF and untouched). Split signals considered: the unmanaged
`/management?project=` half already exists in the tree (uncommitted) and
is extended only by the registered-id redirect; the workbench half and the
login handoff cannot be accepted separately from it because the reported
URL exercises all three. No dependency on another change; nothing else
implements this.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Dashboard router (`VIEW_BY_PATH`, `renderRoute`, `navigateTo`, `popstate`) | `frontend/app.js:32-124` | Same-file param helpers (`managementProjectParam`, `applyManagementProjectParam`) and query-preserving `navigateTo` | None; workbench/login are new call sites of the same pattern | This repo, vendored frontend, no release boundary | **Keep local**: extend the existing router in place |
| Workspace candidate state (`registered` vs `unregistered`) | `src/api/workspace.rs:workspace_registration_state` | Server already reports `state: "registered"` for managed siblings; no server change needed | None | This repo, JSON API (read-only reuse) | **Keep local**: browser reads the existing field |
| Fleet `management` field (`managed`/`observed`/`self`) | `src/api/fleet.rs:gather` | Fleet already routes managed rows to Open and observed rows to Manage | Row action for managed rows carries no URL today | This repo, vendored frontend | **Keep local**: point the existing button at a parameterized path |
| Session gate (`/v1/admin/session`, login page) | `frontend/app.js:loginPage`, `dashboardPage`, `frontend/login.html` | Existing `replace("login.html")` / `replace("index.html")` sites | `next` parameter is new client-side behavior | This repo, vendored frontend | **Keep local**: no server/auth change |

No sibling `common`/`manager` project exists for the dashboard; the only
code that could own this is the vendored `frontend/app.js` router itself.
Decision: **keep local**.

## User Experience and Interface Impact

Actor: the Forge operator in a desktop browser (Chromium-verified;
Safari/Firefox share the same standard APIs: `URLSearchParams`,
`history.pushState`, `popstate`). Entry points: fleet row actions,
pasted/bookmarked deep links, the login page. Primary flows:

- Unmanaged row "Manage" → `/management?project=<id>`, project-scoped
  card first (candidate summary + only that project's preview/confirm),
  bulk hidden, nothing bulk-ticked; reload/back/forward reconcile;
  unknown/empty id ticks nothing with an honest scoped notice.
- Managed row "Open" → `/workbench?project=<id>`, selector set + detail
  loaded; reload/back/forward load the URL's project; unknown id loads
  nothing with an honest notice; plain `/workbench` behaves exactly as
  today (empty selector, no auto-load, hand selection untouched).
- Stale `/management?project=<managed-id>` → `/workbench?project=<id>`.
- Signed-out deep link → login → back to the deep link, project booted.

States: loading (existing spinners/notices), loaded, unknown-id notice,
login `next` validation failure (falls back to `index.html`, never an
open redirect). Keyboard/screen-reader behavior unchanged: real links and
buttons keep labels; notices use the existing live-region elements;
`scrollIntoView` stays `smooth` with the existing `prefers-reduced-motion`
posture. No visual redesign.

## BFS Impact Map

- **Capabilities:** `fleet-manage-deep-link` (new).
- **Users / flows:** operator pastes `/management?project=alethefy`
  signed out and lands managed after login; managed "Open" is shareable.
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count or CLI change. The browser only reads
  the existing fleet `management`/`identity` fields and workspace
  candidate `state`.
- **Integrations / configuration:** none. `scripts/web.sh` symlink staging
  picks up `frontend/` with no change.
- **Callers:** `frontend/app.js` (`renderRoute`, `navigateTo`,
  `openInWorkbench`, `initWorkbench`, `applyManagementProjectParam`,
  `loginPage`, `dashboardPage`); `frontend/login.html` unchanged.
- **Failure / boundary:** unknown/empty id safe on both views; `next`
  outside the route allowlist ignored; unauth API still redirects to
  plain login; signed-out sign-in still lands on `index.html` for bare
  entry.
- **Tests:** new `tests/forge_web_workbench_deep_link_browser.rs` +
  `tests/browser/workbench-deep-link-check.mjs`; extended
  `tests/forge_web_navigation_contract.rs` token test; existing
  `forge_web_manage_deep_link_browser` stays green.
- **Privacy / security:** `next` is same-origin route-path only (open
  redirect refused); no credential, path or hash material enters the URL
  or notices; session handling unchanged.

## Capabilities

- `fleet-manage-deep-link`: every fleet per-project action URL boots the
  view that can manage that project and survives reload, back/forward and
  the login round-trip.

## Non-goals

- No JSON API, auth/session server, registry or catalog change.
- No workbench health-performance work (HANDOFF defect 1 stands).
- No project-retire/death lifecycle (HANDOFF defect 2 stands).
- No visual redesign, no framework, no build step.
