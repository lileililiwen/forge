# Design: Fleet per-project Manage entries land on the managing view

## 1. Decisions

- **(a) Extend the in-tree router (chosen).** `renderRoute` already
  reconciles `?project=` for the management view on every render
  (boot/reload/`popstate`); the workbench and the login handoff join the
  same pattern. No new router, no framework, no second param scheme.
- **URL is the source of truth when a `?project=` is present; operator
  state is never clobbered when it is absent.** A present param that names
  a different project than the loaded one loads the URL's project
  (back/forward/reload honest). An absent param never clears a hand-made
  selection (plain URLs behave exactly as today).
- **Redirect only the already-`registered` management param.** A manual or
  stale `/management?project=<managed-id>` bounces to
  `/workbench?project=<id>`; every other non-selectable candidate state
  and unknown ids keep the honest select-nothing notice so the browser
  never guesses.
- **`next` is a validated same-origin route or it is dropped.** The login
  page only returns to `pathname+search` when `isRoutePath(pathname)` on
  the same origin; anything else (including absolute URLs, `login.html`
  itself, unknown paths) falls back to `index.html`.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/app.js` — workbench param apply, `openInWorkbench` param
    URL, management registered-redirect, login `next` carry/return.
  - `tests/browser/workbench-deep-link-check.mjs` — new browser oracle.
  - `tests/forge_web_workbench_deep_link_browser.rs` — new Rust driver.
  - `tests/forge_web_navigation_contract.rs` — extended token assertions.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/fleet-manage-deep-link/spec.md`).
- **Modules reused unchanged:** `VIEW_BY_PATH` / `viewForPath` /
  `isRoutePath` / `splitRoute` / `navigateTo` / click interceptor /
  `popstate`, `loadWorkbenchDetail`, `ws` candidate table,
  `showWorkbenchNotice`, the session endpoints, `src/web.rs`, `scripts/*`.
- **Must NOT change:** JSON API, auth/session server behavior, registry or
  journal, `command_catalog` rows, static-server allowlist, styles, login
  page markup (only its `app.js` behavior).

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh` symlinks. Tests: Rust
integration oracles driving the real `forge` binary (`cargo test --test
<name>`) plus a Node 24 + Playwright 1.63 Chromium harness from
`tests/browser/` (exit `0` verified / `1` failed / `2` UNVERIFIED when the
browser stack is unavailable — never recorded as a pass). Local
conventions: `forge_bin()`, `free_port()`, `copy_dir()`, `ChildGuard`,
`wait_for_port` in the Rust driver; `note`/`fail` logging in the harness.

## 4. User experience and interface

Actors/goals/entry as in the proposal UX section. Exact behavior:

| URL | State | Result |
|---|---|---|
| `/management?project=<onboardable>` | signed in | scoped card leads (summary + single-project preview/confirm), bulk hidden, nothing bulk-ticked |
| `/management?project=<registered>` | signed in | `navigateTo(/workbench?project=<id>)`; detail loads there |
| `/management?project=<unknown/empty>` | signed in | scoped safe notice, nothing ticked, no preview |
| `/workbench?project=<managed>` | signed in, selector ready | selector set + `loadWorkbenchDetail(id)` |
| `/workbench?project=<unknown>` | signed in, nothing loaded | empty state stays + `No managed project matches … — nothing was loaded.` |
| `/workbench?project=<unknown>` | signed in, project loaded | loaded project kept + same notice |
| `/workbench` (no param) | any | unchanged: no auto-load, hand selection untouched |
| deep link (any view) | signed out | `login.html?next=<enc path+query>` → after sign-in, back to the deep link |
| `login.html?next=<evil>` | any | `next` fails validation → `index.html` |

Bookkeeping: `wbAutoParam` (last URL-applied workbench id) prevents
re-fetch on unrelated re-renders; a changed param re-loads. Management
keeps its `ws.autoSelected` protocol untouched. Focus order: workbench
load keeps its existing `scrollIntoView`; management keeps its row
scroll; login focus unchanged. Notices render into the existing
`#ws-notice` / `#workbench-notice` live regions.

## 5. Behavioral model

- `workbenchProjectParam()`: trimmed `project` query value or `null`.
- `applyWorkbenchProjectParam()`: no-op unless the workbench selector has
  managed options (fleet not yet loaded → `initWorkbench` calls apply
  after populating). `id == null` → return (plain URL untouched).
  `id === workbench.id` (or `=== wbAutoParam` while loading) → return.
  Option exists → `select.value = id`, `wbAutoParam = id`,
  `loadWorkbenchDetail(id)` (async; failures render the existing
  workbench error state). No option → `wbAutoParam = null`,
  `showWorkbenchNotice("No managed project matches “<id>” — nothing was loaded.")`,
  selection/body untouched.
- `openInWorkbench(identity)`: `navigateTo("/workbench?project=" +
  encodeURIComponent(identity))` only; `renderRoute` applies the load.
- Management registered-redirect: inside `applyManagementProjectParam`,
  when the matched candidate has `state === "registered"`,
  `navigateTo("/workbench?project=" + encodeURIComponent(id))` and
  return (workbench apply takes over; no loop: workbench never bounces
  back for a managed id).
- Login: `loginNextTarget()` returns validated `pathname+search` or
  `null`. `dashboardPage` unauthenticated →
  `replace("login.html?next=" + encodeURIComponent(pathname+search))`
  (bare entry yields `next=/projects`-style values only when the current
  path is a route; entry at `/` yields `next=/index.html`? No: dashboard
  boot only runs on the shell; `/` serves `login.html` directly, so the
  dashboard `next` always carries a real view path). `loginPage`:
  authenticated → `replace(next || "index.html")`; submit success →
  `assign(next || "index.html")`.

## 6. Contract and compatibility

No wire, storage or CLI contract changes. URL contracts (new, browser
only): `/workbench?project=<id>` boots the managed project;
`/management?project=<registered-id>` redirects to the former;
`login.html?next=<route>` returns after sign-in. Backward compatible:
bare `/workbench`, `/management`, `login.html` and every existing link
behave exactly as before; unknown/empty params and invalid `next` degrade
to current behavior plus an honest notice where one already exists.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Workbench param names an unmanaged/observed id | No option exists → unknown-id notice, nothing loaded (operator must onboard it via management first) |
| Workbench detail fetch fails (doctor/checker, 404, offline) | Existing `loadWorkbenchDetail` error path: body hidden, empty shown, notice with the server message |
| Fleet load fails (selector empty) | Param apply stays deferred; plain workbench error surface unchanged |
| `next` is absolute/cross-origin/unknown path | Dropped → `index.html`; never an open redirect |
| Session expires mid-view | Next dashboard boot carries the then-current deep link to login and back |
| Rapid back/forward across params | Each `popstate` re-applies the current param; in-flight detail loads resolve in order, last URL wins by `workbench.id` check |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean.
- New `cargo test --test forge_web_workbench_deep_link_browser` green in
  real Chromium: Open-button URL + boot; direct boot; reload keeps;
  unknown id safe (nothing loaded + named notice); back/forward across
  two managed ids; registered-id management→workbench redirect;
  signed-out deep-link → login → deep link booted (management tick and
  workbench detail).
- Extended `cargo test --test forge_web_navigation_contract` token test
  (workbench param + login `next` + Open param URL tokens).
- No-regression: `forge_web_manage_deep_link_browser`,
  `forge_web_command_catalog_contract`,
  `forge_web_maintainer_surface_contract`, `cargo test --lib
  command_catalog`, `cargo test --bin forge`.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- Manual: throwaway mirror of live (registry copy + `/home/paul/code`
  root + throwaway ports) driven in Chromium for the exact reported URL
  signed-out and signed-in, both views.

## 9. Decision ledger

- **Resolved:** workbench param acts only when present; absent param
  never clears hand state (avoids destroying in-progress confirm flows on
  plain `/workbench` visits).
- **Resolved:** managed "Open" stays a `<button>` (no markup/a11y churn);
  only its navigation target gains the project param.
- **Resolved:** redirect (not notice) for the registered-id management
  param, because the workbench is the view that can manage that project.
- **Resolved:** `next` validated against the existing `isRoutePath`
  allowlist — the same list the click interceptor trusts.
- **Deferred (non-goals):** workbench health latency, project
  retire/death, visual redesign.
- **Blockers:** none.
