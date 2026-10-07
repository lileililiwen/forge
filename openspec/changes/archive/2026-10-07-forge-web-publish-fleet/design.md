# Design: forge-web-publish-fleet

## Implementation boundary

Extend Forge's existing Rust 2021 API fleet read model. Inspect
`src/api/fleet.rs`, `src/registry/mod.rs`, `src/api/admin.rs`
(`redact_local_paths`), `src/api/ui/data.rs` (`pick_last_publish`), the
`operations` schema in `src/registry/mod.rs`, and `frontend/`. Keep the
JSON API and the browser transport separate: aggregation stays in the API,
all HTML/CSS/JS stays in `frontend/`. Do not modify `src/publish/*`,
`src/delivery/*`, the bearer `/v1` routes, or any provider adapter.

## Language and runtime

Rust 2021, MSRV 1.87, the existing blocking HTTP/1.1 API and SQLite
registry, browser-native HTML/CSS/JS. No new runtime service, dependency,
subprocess or network call. Verify with `cargo fmt --check`,
`cargo check --all-targets`, the focused Rust contract tests,
`openspec validate --all --strict --no-interactive`, and a browser/API
smoke against a **copy** of the operator's registry.

## Ownership and shared code

| Concern | Owner | Decision |
|---|---|---|
| Fleet aggregation, source descriptors, redaction, rendering | `src/api/fleet.rs` | extend in place |
| `operations` SQL projection | `src/registry/mod.rs` | add one bounded method |
| Frontend source label/filter/chip | `frontend/app.js`, `frontend/index.html` | extend in place |
| Path redaction | `src/api/fleet.rs` (private copy) | mirror the workbench discipline; do not move the two existing copies |

No shared extraction. `fleet.rs` already owns aggregation and must not
import a private helper from `workbench.rs`; a local `redact_local_paths`
copy keeps the module boundary intact, matching the existing duplication
between `admin.rs` and `workbench.rs`.

## Behavioral model

The fleet gains one source:

| Source | id | kind | Data | Always on? |
|---|---|---|---|---|
| Publish history | `published` | `forge-publish-history` | local `operations`, most recent `publish` row per `project_id` | yes, disable with `FORGE_PUBLISH_HISTORY=0` |

Projection (per distinct `project_id`, newest `op_id` wins):

| Journal column | Fleet field | Rule |
|---|---|---|
| `state` | row `state`, `publish.state` | verbatim (`done`/`failed`/`skipped`/`pending`) |
| `started_at` | row `updated_at`, `publish.started_at` | verbatim RFC3339 |
| `finished_at` | `publish.finished_at` | verbatim or absent |
| `detail` | `publish.detail` | whitespace-token path-redacted |
| `queue_id` | `publish.target` | verbatim (fleet run id) or absent |
| `revision` | `publish.revision` | verbatim when present |
| `build_status` | `publish.build_status` | verbatim when present |
| `run_status` | `publish.run_status` | verbatim when present |
| `container_identity` | `publish.container_identity` | verbatim when present |
| `detail` | `publish.healthy` | `bool` when `healthy=true/false` is present, else `null` |
| `detail` | `publish.stages` | `int` when `stages=N` is present, else `null` |

Merge rule:

- Build the set of locally registered ids while projecting the registry
  rows.
- For each published project id in that set, **attach** `publish` to the
  existing `registry` candidate and append a `("publish", state)` evidence
  entry. No second row is emitted.
- For each published project id **not** in the set, push a new candidate:
  `source = Published`, `management = Observed`, `state = <op state>`,
  `updated_at = started_at`, `freshness` from `started_at` vs
  `FORGE_FLEET_MAX_AGE_SECONDS`, `publish = Some(...)`,
  `capabilities = []`.
- The self row is handled by the existing merge (a published `forge`
  row would attach to self exactly like a registry row; no duplication).

Sorting is unchanged (normalized name, then identity). Conflict detection
is unchanged and is never triggered by this source for a registered
project because the row is merged, not duplicated.

Rows are bounded by `FORGE_PUBLISH_HISTORY_LIMIT` (default 200, clamp
1..=1000) applied at the SQL `LIMIT`, so a long journal cannot pin the
renderer.

## Contract and compatibility

`WEB_FLEET_CONTRACT_VERSION` is bumped `forge-web-fleet/0.1.0` →
`forge-web-fleet/0.2.0`. The response stays backward compatible: existing
fields and sources are unchanged; only `sources[]` gains a
`published` descriptor, rows gain an optional `publish` object, and
`summary.by_source` gains `published`. `summary.total` counts the added
rows. No route, auth or bearer contract changes.

## Failure and boundary policy

| Condition | Behaviour |
|---|---|
| `FORGE_PUBLISH_HISTORY` = `0`/`false`/`off` | source descriptor `unconfigured`, reason "publish history projection is disabled by FORGE_PUBLISH_HISTORY"; no rows |
| No publish rows (fresh registry) | source descriptor `available`, `count 0`; zero rows; other sources untouched |
| Registry unreadable | source descriptor `unavailable`, safe reason; other sources untouched |
| Legacy row with `NULL` phase columns | projected with those fields absent; never an error |
| `detail` contains an absolute path | replaced with `[local path]` before serialization |
| Registered project also published | merged into the managed row; no false conflict |
| Published id valid but no local profile | `profile` absent; observed row |

The projection is read-only and bounded. It never writes the registry,
the journal or a project file, never scans the filesystem, and never calls
the network.

## Verification oracle

- `tests/forge_web_publish_fleet_contract.rs` drives the real
  `GET /v1/admin/projects` handler with a throwaway registry seeded
  through `Registry`, covering:
  1. self-only registry with no publish rows → `published` source
     `available, count 0`.
  2. one publish-only project → exactly one observed row with
     `source=published`, `management=observed`, `capabilities=[]`, and the
     projected `state`/`publish`.
  3. a registered project that is also published → one row, still
     `managed`, `capabilities` contains `inspect`, `publish` attached, no
     `conflict`.
  4. most recent publish wins (two ops, older `failed`, newer `done`).
  5. `FORGE_PUBLISH_HISTORY=0` → source `unconfigured`, no published rows,
     other sources intact.
  6. `detail` carrying an absolute path → `[local path]`, no
     `/home/` or temp root in the response.
  7. anonymous request → 401 with no rows/sources.
- `forge_web_fleet_contract.rs` stays green (its fixtures write no
  `publish` operations, so no new rows).
- Browser/API smoke against a copy of the operator's real registry:
  authenticate, fetch `/v1/admin/projects`, assert the published-only
  projects appear as observed rows and no absolute path appears in the
  body.

## Decision ledger

- Resolved: read the local journal only; no Mac round trip.
- Resolved: enabled by default (it is the local Forge-owned registry, not
  an external source) with an explicit opt-out.
- Resolved: merge into existing registered rows rather than duplicate, to
  avoid a false identity conflict and keep `inspect`.
- Resolved: published-only rows are `observed` (inspection only), matching
  inventory/workspace sources.
- Deferred to `forge-web-project-management`: any project mutation from the
  browser.
- Deferred to `forge-web-project-status`: live `doctor`/`check`/readiness.
- No blockers.

## Requirement traceability

| Requirement | Design decision / boundary | Scenarios | Task IDs | Verification oracle |
|---|---|---|---|---|
| Local publish history is projected into the fleet | bounded `GROUP BY` over `operations`, merged by identity | published-only row; registered+published merge; newest wins; empty journal; disabled | 2.1–2.5 | contract test cases 1–5 |
| Published rows are truthful and path-safe | observed-only management; redacted `detail`; honest source states | path redaction; unavailable/disabled source | 2.1–2.5, 3.2 | contract cases 5–7 |
