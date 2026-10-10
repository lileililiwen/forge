# Proposal: Workbench health latency

## Why

`GET /v1/admin/projects/{id}` — the single request that opens a project
in the workbench — runs the external DriftWatch checker **and** a full
doctor pass inline, in the request path, on every open. Browser-measured
on `argoscope`: **27,242 ms** for that one request; direct `curl` ~6.5 s
varying with the project's toolchain. The frontend's `request()` helper
has no timeout, so the workbench reads as hung for the better part of
half a minute. This is very likely the dominant cause of the workbench
reading as "broken" rather than as a design problem (HANDOFF 2026-10-08,
Defect 1).

An external-checker run and a full doctor pass are **not** read
operations and should not be inline in a GET.

## What Changes

- The detail GET keeps its route, contract version, shape and journal
  behavior; its `health` projection becomes a **fast local pass**:
  `run_doctor` with no policy outcome (local manifest/profile/feature/git
  findings only — millisecond-scale, two `git` probes at most) plus one
  explicit `unavailable` finding (`policy-deferred`) so the row never
  claims a pass it did not compute. A locally-clean project reports
  state `deferred` (new, documented value) instead of `healthy`; local
  problems still report `issues`; `stale` and `unavailable` keep their
  exact meanings.
- The live pass moves behind one explicit operator action: new
  `POST /v1/admin/projects/{id}/health/refresh` runs DriftWatch +
  doctor exactly as the GET does today and returns the full health
  document (states `healthy`/`issues`/`stale` as today, no deferred
  finding). Read-only effect: no confirm/digest binding, no journal row
  — the response itself is the evidence.
- The workbench health card renders the `deferred` state with its
  reason, offers **Run full health check** (progress state, re-renders
  the card), and renders the `policy-deferred` row with that button
  instead of a `remediate plan` shortcut. `HEALTH_LABELS`/`HEALTH_BADGE`
  gain the new state; all other states render byte-identically.
- Sentinel-oracle contract test: with `FORGE_DRIFTWATCH_BIN` pointed at
  a script that touches a file, the detail GET leaves the file
  untouched while the refresh route touches it.

## Package Boundary and Split Assessment

One independently verifiable outcome: opening a project is fast and the
full check is one explicit click away, with no invented passes. The GET
change, the refresh route, the card rendering and the sentinel test
share one surface (workbench detail), one journey and one oracle;
splitting them ships a fast GET that can never run the full pass, or a
refresh route nothing calls. No schema, catalog-shape or Core change
splits off: the catalog gains one row for the new route, nothing more.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `workbench-health-latency` (**this**) | Fast workbench open + explicit full-check refresh with identical safety properties | Forge `src/api/workbench.rs` + router/dispatch + catalog row + `frontend/` health card / Rust+JS | Same routes/shapes plus one new POST; `forge-project-workbench/0.1.0` version unchanged | `maintainer-plugin-platform` (maintain surface precedent) | Sentinel-binary contract test + updated workbench contract pin |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Health projection | `src/api/workbench.rs` `detail`/`build_health` | Shape, path stripping, unavailable branch | Runs subprocesses inline | `src/api` owns the projection | **extend shared owner** — fast local pass, same shape |
| Doctor runner | `src/doctor/runner.rs` `run_doctor` | `None` policy outcome already omits policy findings (assessment.rs:424) | None — designed for it | `src/doctor` owns the runner | **reuse unchanged** |
| DriftWatch adapter | `src/policy/driftwatch.rs` `run_driftwatch` + `FORGE_DRIFTWATCH_BIN` override | Full live pass + env override (test hook) | Slow by nature (external process) | `src/policy` owns the adapter | **reuse unchanged**, move behind refresh |
| Router/dispatch | `src/api/router.rs` segment table, `Route` enum, `admin/deploy.rs` dispatch | Maintain-surface precedent (`maintain`/`classify` arms) | New arm needed | `src/api` owns routing | **extend shared owner** — one new arm |
| Health card | `frontend/app.js` `renderHealth`, `HEALTH_LABELS/BADGE` | Existing states, findings list, remediate shortcut | No `deferred` state, no refresh control | `frontend/` owns presentation | **extend shared owner** — new state + button |

No sibling checkout is touched; no host folder appears in code.

## User Experience and Interface Impact

Actor: the operator opening a project. Before: up to ~27 s of apparent
hang. After: the workbench opens immediately with locally-verified
health; when local checks are clean the card reads **Deferred** with
"the live policy check has not run for this view" and a **Run full
health check** button showing progress while the live pass runs, then
re-rendering the card. Blocked/unavailable/failed states keep their
honesty and wording. Responsive/keyboard/a11y behavior unchanged in kind
(same landmarks, live regions, focus hooks); the new button reuses
`.button .button-quiet` and the card's `role=status` result pattern.
`UI/UX: N/A` does not apply — the card copy and control are part of the
change and are covered by the browser contrast/keyboard oracle if the
drive is extended; at minimum the frontend source contract pins the new
tokens.

## BFS Impact Map

- **Capabilities:** `workbench-health-latency` (new). No API version,
  code, status, digest or journal change.
- **Users / flows:** every workbench project open; every explicit
  full-check refresh.
- **Contracts / data / persistence:** `health.state` gains `deferred`;
  findings gain id `policy-deferred`; refresh route returns today's full
  document. No schema change. No journal rows for reads (as today).
- **Callers:** `detail` (same route), new refresh route, health card.
- **Failure / boundary behavior:** unreadable dir → `unavailable` as
  today; refresh with missing checker → full document with `unavailable`
  policy finding (today's behavior, now on demand); refresh failure →
  honest error, card keeps prior state.
- **Tests:** sentinel-oracle contract test (new); workbench contract pin
  extended with `deferred`; frontend source contract pins new tokens;
  all existing suites unchanged.
- **Privacy / security:** strictly fewer subprocesses on the read path;
  no new path/credential surface (refresh resolves server-side as today).

## Capabilities

- `workbench-health-latency`: fast workbench open with explicit,
  on-demand full health check.

## Non-goals

- No caching layer, no per-(project, commit) store, no schema change.
- No change to `forge doctor`, `forge gate`, `forge check` or any CLI
  behavior.
- No retire/death lifecycle (HANDOFF Defect 2 — needs owner ruling).
- No frontend request timeout (separate concern, not required once the
  GET is fast).
- No removal of information: the full live report remains one click
  away with identical content.
