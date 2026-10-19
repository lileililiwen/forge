# Design: Authenticated web workflow for Forge modules

## Implementation boundary

Implement in the Forge repository. Extract validated module discovery into a
public library-owned module catalog so `src/main.rs` CLI dispatch and `src/api`
use one resolver. Add module list/status/run handlers below
`src/api/admin/`, register their exact paths in `src/api/admin/routes.rs` and
the router, and extend the typed command catalog. Add a Modules navigation
entry, panel and rendering/actions to `frontend/index.html`, `frontend/app.js`
and `frontend/styles.css`. Keep the existing Gate engine and module-owned
`.project.json` and Gate declarations authoritative.

Do not modify sibling projects, their CI or Gate declarations. Do not add a
generic shell, arbitrary path, command, provider, or network input to the API.

## Language and runtime

- Rust stable workspace as pinned in `rust-toolchain.toml`, serde/serde_json
  for typed JSON, and the current `crate::gate` runtime.
- Standalone HTML, CSS and vanilla JavaScript served by `forge web serve`.
- Verification uses focused `cargo test`, `scripts/check-tests.sh`, the
  repository browser contract harness, `rustfmt --check`, OpenSpec strict
  validation, and local `forge gate`.
- Browser support follows the existing portal target: current Chromium desktop
  and narrow viewport testing through the existing browser harness.

## Ownership and shared code

Create a Forge-owned library module (recommended path `src/module_catalog.rs`)
with a serializable `ModuleEntry` and functions to discover modules from an
explicit root and resolve an ID. It owns ID validation, metadata parsing,
duplicate detection, exact immediate-child containment, Gate declaration
containment, stable sort order and path-free projection. The CLI derives its
source root from the executable as it does today and renders the same data.
The API uses that same executable-derived root; no browser or request header
selects a root.

Gate execution and evidence remain owned by `crate::gate`. The API reuses
`run_gate`, `save_evidence`, `load_latest_evidence`, revision capture,
freshness, verdict and summary functions. The Forge root Registry journals a
real module Gate result under the module ID, matching the CLI workflow. The
standalone UI consumes only versioned API envelopes; no shared frontend or
cross-language module is introduced.

## User experience and interface

Add a `Modules` primary navigation destination at `/modules`. It contains:

1. A short description that this page covers modules declared in the running
   Forge source checkout and invokes each declaration's existing Gate checks.
2. A list/table of modules with ID, stack, profile, relative path, last
   aggregate, freshness and an Open action.
3. A detail region for the selected module with Fast/Full profile selection,
   Run Gate action, revision and observed time, plus per-check state/details.
4. Inline status and error regions for loading, empty, unavailable,
   unauthorized, running, valid FAIL/REVIEW_REQUIRED, and request/runtime
   failures. Preserve the latest known evidence on request failure.

On desktop use a readable table/list and adjacent detail card; below 640px
stack module rows and detail fields vertically without horizontal page
overflow. Use semantic headings, a labelled profile select, buttons with
explicit names, visible keyboard focus, Enter/Space support, `aria-live` for
run transitions and results, and focus restoration to the selected module
after a refresh. Do not announce every per-check detail repeatedly; announce
the aggregate and place detailed checks in a labelled region. No animation is
needed for data loading or completion; honor reduced motion for any existing
portal transition.

## Behavioral model

| State/event | Behavior | Persistence |
|---|---|---|
| Page open | Fetch the module catalog and latest status; each read is admin-session gated | None |
| No module declarations | Return an empty list; UI shows a truthful empty state | None |
| Source root or catalog invalid | Return typed unavailable/invalid response; do not return partial modules or absolute path | None |
| Select module | Fetch latest Gate status by validated module ID | Read only |
| Run Gate | Validate profile, resolve ID again, claim per-module in-process run guard, then run existing Gate engine | Save Gate evidence under that module and journal one Forge `gate` operation on a real evidence result |
| Gate returns PASS/FAIL/REVIEW_REQUIRED | Return actual parsed evidence and freshness; FAIL is a recorded result | Atomic module evidence plus Registry operation |
| Invalid runtime, timeout, save failure | Return unavailable/typed error; retain previous evidence | No new evidence; journal only where existing CLI/API contract allows |
| Same module already running | Return 409 `module-gate-in-progress` without starting another execution | None |
| Different module run | It may proceed concurrently; the run guard is keyed by module ID | Independent module evidence and operations |

The run guard is process-local and held for the duration of synchronous Gate
execution. It is removed on every return path by an RAII guard. Restarting the
server clears only active-run state; it does not change persisted evidence.

## Contract and compatibility

Use API contract `forge-module-workflow/0.1.0`:

- `GET /v1/admin/modules` returns `{contract, modules}`. Each module has
  `id`, `stack`, `profile`, Forge-relative `path`, and a latest Gate summary
  (`aggregate`, `freshness`, `profile`, `revision`, `observed_at`, `checks`)
  or an explicit `unverified` state when no evidence exists.
- `GET /v1/admin/modules/{id}/gate/status` returns the same module Gate
  evidence envelope used after a run; absent evidence is `unverified`, never
  PASS.
- `POST /v1/admin/modules/{id}/gate/run` accepts only a JSON object with
  optional `profile: "fast" | "full"`; omitted profile means `full`. It
  returns the real Gate evidence envelope. A Gate blocked verdict is HTTP 200
  with its actual aggregate; malformed profile is 400; duplicate run is 409;
  missing/invalid module source is 503; unauthenticated/unauthorized follows
  the existing admin guard status.

The route matcher accepts only a single ID path segment. Every request
re-discovers and validates the module before operating. The API serializes no
absolute path, environment dump or unbounded raw runtime output; scrub module
root paths from errors and bound the wire evidence to the existing Gate
contract. Existing `/v1/admin/projects/{id}/gate/*` responses do not change.
CLI output formats and exit codes remain compatible.

## Failure, authorization and concurrency policy

All routes use the existing global Forge admin-session guard. Missing or
invalid sessions receive the established 401 response and no module data.
Only declared IDs discovered directly under canonical `modules/` resolve;
traversal, malformed metadata, duplicate IDs, symlink escape, missing source
root and unknown ID fail closed. A malformed catalog fails as a whole rather
than presenting a partial or misleading module list. A binary-only runtime
without in-tree module assets returns unavailable and does not scan parent
directories or sibling checkouts.

POST accepts only a bounded body and the two Gate profiles. No user-provided
path, command, timeout or gate config is accepted. A repeated request while a
same-module run is active returns 409 and tells the UI to refresh status; it
does not queue or duplicate work. A browser disconnect does not cancel an
already-started Gate run, matching the synchronous existing Gate endpoint.
Atomic evidence save and Registry journal outcomes follow the existing Gate
transport semantics. Errors preserve prior evidence and the UI keeps its last
known status visible with a retry action.

## Verification oracle

- Core module catalog tests cover stable order, empty catalog, malformed and
  duplicate metadata, invalid IDs, missing Gate declaration, each supported
  declaration location, and symlink/path escapes.
- API contract tests cover all routes, session rejection, invalid profile,
  unknown module, unavailable source, exact response fields, no absolute path,
  no evidence on rehearsal/unavailable runtime, PASS and FAIL evidence, and
  same-module concurrency refusal.
- Command catalog tests assert both module CLI verbs map to the Modules route
  and top-level CLI catalog parity remains exhaustive.
- Browser tests cover auth, list/detail, empty/unavailable/error states,
  fast/full run, actual PASS and FAIL rendering, duplicate disablement,
  keyboard navigation, screen-reader announcements, and 360px/1280px layout
  without horizontal overflow.
- Run focused Rust tests, all `scripts/check-tests.sh`, module Python tests if
  shared module declarations change, JSON validation, formatting, OpenSpec
  preflight/strict validation, diff check, then Gate dry-run and bounded full
  Gate before archive.

## Decision ledger

- Resolved: the portal is the human entry point; both server and browser
  portions ship in this same capability.
- Resolved: the running Forge executable locates only its own source checkout;
  there is no configurable root or sibling fallback.
- Resolved: Gate execution is synchronous like the existing project Gate API;
  same-module runs are rejected while one is active.
- Resolved: module FAIL/REVIEW_REQUIRED is evidence and remains visible as an
  actual blocked result, not an API transport failure.
- Deferred: hosted CI for module repositories remains owned by each module's
  CI; the page reports local Forge Gate only.
- No blockers.

## Requirement traceability

| Requirement | Design boundary | Scenarios | Tasks | Verification |
|---|---|---|---|---|
| Authenticated declared module inventory | Shared Forge module catalog + GET list | populated, empty, invalid/unavailable, unauthorized | 2.1–2.3, 3.1 | Core and API contract tests |
| Per-module Gate evidence and execution | Existing Gate engine + status/run routes | absent, PASS, FAIL, unavailable, duplicate run | 2.2–2.4, 3.1–3.2 | API evidence and concurrency assertions |
| Usable accessible web workflow | Standalone Modules page and catalog mapping | load, empty, errors, run, keyboard/mobile | 2.5, 3.2 | browser/a11y/responsive contracts |
