# Proposal: Show published projects in the web fleet

## Why

The operator publishes a large portfolio to the Mac server through
`forge publish` (the Jenkins / remote-compose chain) but most of those
projects were never registered in the local `projects` table. The web
portal's fleet (`GET /v1/admin/projects`) therefore shows only the handful
of locally registered projects and omits everything the operator actually
ships. The operator's own words:

- "i have 70+ project, and i publish a lot to the mac server"
- "i can still not manage project"
- "find the project status"
- "the project list is wrong"

Measured on the operator's real registry
(`~/.local/share/forge/registry.db`, copied read-only for inspection):

- `projects` table: **7** rows.
- `operations` table: **3,109** `publish` rows across **25** distinct
  `project_id`s, most of which are absent from `projects`.
- Most recent `publish` row per project carries the deploy outcome in
  `state` and the human evidence in `detail` (for example
  `fleet stages=4 healthy=true` or
  `publish all summary via remote-compose: healthy=true stages=4`).
  `revision` / `build_status` / `run_status` / `container_identity` are
  present as columns and populated by newer rows, `NULL` on legacy rows.

The data already exists locally; the fleet read model just never projected
it. This change adds a bounded, read-only projection of the local publish
journal into the fleet so the portal list matches what the operator ships.

## What Changes

- Add a fourth bounded source to the fleet envelope:
  `id = "published"`, `kind = "forge-publish-history"`, read from the
  **local registry** `operations` table (the same `db_path` the API already
  opened) — no external path, no scan.
- Project the **most recent** `publish` operation per `project_id`
  (`kind IN ('publish','publish.github')`), newest by `op_id`), surfacing:
  `state`, `started_at`, `finished_at`, `detail` (path-redacted),
  `queue_id`, `revision`, `build_status`, `run_status`,
  `container_identity`, plus a derived `healthy` / `stages` boolean/int
  parsed from `detail`.
- **Merge, never duplicate**: when a published project id is already a
  locally registered project, attach the publish projection to that
  managed row. Only ids with no local registration become standalone
  `observed` rows. This keeps the existing managed row's `inspect`
  capability and avoids a false identity conflict.
- Standalone published-only rows are `management = observed`: inspection
  only, no Forge mutation link, exactly like the inventory/workspace
  sources.
- Gate the projection with `FORGE_PUBLISH_HISTORY` (default **enabled**;
  `0`/`false`/`off` disables) and bound it with
  `FORGE_PUBLISH_HISTORY_LIMIT` (default 200, clamped 1..=1000). Freshness
  reuses `FORGE_FLEET_MAX_AGE_SECONDS`.
- Extend the frontend: a `Published (Mac)` source label/filter and a
  publish chip (state + health + redacted target/revision) on the row.
- Every declared absolute path in `detail` is replaced with
  `[local path]` before serialization; the existing no-path contract test
  is extended to the publish source.

## Package Boundary and Split Assessment

This is tier 1 of the operator's three-part gap report and is the smallest
independently verifiable unit:

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-publish-fleet` (**this**) | The fleet shows every project the local publish journal knows, with its most recent publish outcome | Forge `src/api/fleet.rs` + `src/registry` / Rust, `frontend/` | New `published` source on `GET /v1/admin/projects`; read-only local `operations` projection | `forge-web-project-fleet`, `forge-web-project-deployment` | `tests/forge_web_publish_fleet_contract.rs` + `forge_web_fleet_contract` |
| `forge-web-project-management` (follow-on) | Create/import/register/upgrade/release/publish a project from the browser | Forge `src/api` + `src/release` + `src/publish` / Rust | New admin mutation routes behind confirm→digest | this package | management contract test |
| `forge-web-project-status` (follow-on) | Run real `forge doctor` / `check` / `readiness` for one project and a fleet-wide readiness tile | Forge `src/api` / Rust | New read-only status route + workbench health card | this package | status contract test |

The publish-history projection is the only one of the three that is pure
read + aggregation on already-persisted local data, so it carries no
subprocess, network, credential or write surface and is safe to lift first.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling to adopt
from and no shared extraction is in scope. The reconnaissance is intra-repo:

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|---|
| Fleet aggregation | `src/api/fleet.rs` `gather` / `build_envelope` / `SourceDescriptor` | The exact source/merge/redaction envelope | No publish source | `src/api/fleet.rs` | **extend owner** — add the source, do not fork the envelope |
| Operations read model | `src/registry/mod.rs` `OperationEntry`, `recent_operations`, `operations_for_project` | Column projection + legacy-column migration | No per-project "latest publish" query | `src/registry` | **extend owner** — add one bounded `GROUP BY` query |
| Last-publish UI pick | `src/api/ui/data.rs` `pick_last_publish` | Existing definition of "last publish" (`publish`/`publish.github`) | Only reads projects already in `projects` | `src/api/ui/data.rs` | **reuse the vocabulary**, new query covers the missing ids |
| Path redaction | `src/api/admin.rs` `redact_local_paths`; `src/api/workbench.rs` `redact_local_paths` | Whitespace-token absolute-path redactor | Two copies already exist | `src/api` | **reuse the workbench discipline** locally in `fleet.rs` |
| Frontend source labels | `frontend/app.js` `SOURCE_LABELS`/`SOURCE_BADGE`, `frontend/index.html` filter | Catalog-driven renderer | No `published` entry | `frontend/` | **extend owner** |

No shared extraction or sibling edit is authorized; every decision is an
in-repo extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-fleet` (modified: the fleet SHALL
  also project the local publish journal as a bounded source).
- **Users / flows:** the single global admin operator opening **All
  projects** in the portal.
- **Contracts / data / persistence:** no schema change. One new source
  descriptor, one new per-row `publish` object, one new
  `summary.by_source.published` key. `fleet_contract` version is bumped
  `forge-web-fleet/0.1.0` → `forge-web-fleet/0.2.0` because a new source
  and row field are added.
- **Integrations / configuration:** `FORGE_PUBLISH_HISTORY` (default on),
  `FORGE_PUBLISH_HISTORY_LIMIT` (default 200), reuses
  `FORGE_FLEET_MAX_AGE_SECONDS`.
- **Callers:** `src/api/fleet.rs` (gather/render/summary),
  `src/registry/mod.rs` (new query), `frontend/app.js` +
  `frontend/index.html` (source label/filter/chip), the fleet contract
  test.
- **Failure / boundary behavior:** disabled → `unconfigured` with a safe
  reason; legacy registry with no publish rows → `available, count 0`
  (healthy empty); unreadable registry → `unavailable` with a safe reason,
  other sources untouched; malformed/legacy `detail` → still projected,
  never a crash; `detail` absolute paths → `[local path]`; existing
  managed project → merged, never a false conflict.
- **Tests:** new `tests/forge_web_publish_fleet_contract.rs`; the existing
  `forge_web_fleet_contract.rs` must stay green (no publish rows in those
  fixtures means no new rows).
- **Dependencies / compatibility:** additive to the archived
  `forge-web-project-fleet` and `forge-web-project-deployment` shapes; no
  bearer `/v1` route changes.
- **Privacy / security:** only the local Forge-owned registry is read;
  responses are path-redacted; no new write, subprocess or credential
  surface.

## Capabilities

- `forge-web-project-fleet` — modified: the authenticated fleet projects
  the most recent local publish operation per project as an additional
  bounded, read-only source, merged into existing managed rows and
  otherwise observed.

## Non-goals

- No Mac/Jenkins round trip, no live `forge publish`, no provider or
  OpenPanel delivery call. Only the local journal is read.
- No mutation of `operations`, `projects` or any project file.
- No new persistence schema and no generic shell/argv execution.
- No change to the bearer `/v1` deploy route or the delivery/portfolio
  share surface.
