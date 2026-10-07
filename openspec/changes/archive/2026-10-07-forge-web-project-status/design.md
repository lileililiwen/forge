# Design: Read-only project status and fleet readiness

## 1. Implementation boundary

- **Repository / project:** this Forge repository (`/home/paul/code/forge`),
  the `forge` crate. No sibling project is touched.
- **Modules changed:**
  - `src/api/status.rs` (**new**) — the `forge-project-status/0.1.0`
    contract, the two route constants, `project_status`, `fleet_status`, and
    the sub-check mapping.
  - `src/api/mod.rs` — `mod status;`, two `Route` variants, two router arms,
    the `required_permission` entry and the admin short-circuit dispatch, and
    the unreachable-dispatch arms.
  - `src/api/admin.rs` — the two `handle` arms delegating to `status::*`
    through `guarded`.
  - `src/api/workbench.rs` — widen `resolve`, `Resolved` and
    `redact_local_paths` to `pub(super)` so the status module reuses the exact
    validated-id resolution and path-redaction discipline; no behavior change.
  - `src/api/command_catalog.rs` — add the two routes to
    `IMPLEMENTED_WEB_ROUTES`; recatalogue `check` to `web`; repoint
    `fleet.status`; update the pinned web list in the in-source tests.
  - `frontend/app.js` + `frontend/index.html` — a workbench status card and a
    dashboard fleet readiness tile, rendered from the typed endpoints.
- **Modules reused unchanged:** `src/doctor/mod.rs` (`run_doctor`,
  `DoctorReport`, `FindingStatus`), `src/checker/mod.rs` (`build_document`,
  `truncate_alerts`, `AlertSeverity`), `src/governance.rs` (`inspect`),
  `src/profile/mod.rs` (`inspect_profile`, `ProfileSupportStatus`),
  `src/readiness/mod.rs` (`matrix_profile_ids`, `ReadinessStatus`), and
  `src/core/mod.rs` (`validate_project_id`).
- **Must NOT change:** the CLI dispatch in `src/main.rs`, the workbench
  detail/plan/apply routes and their behavior, the bearer `/v1` routes, the
  MCP tools, any Core doctor/checker/governance/readiness semantics,
  `API_CONTRACT_VERSION`, the catalog row count (227), and the DriftWatch
  adapter invocation of CLI `doctor`.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (plus `cargo test --bin forge` for
  the in-source catalog tests). Format: `cargo fmt`.
- Frontend: plain script under `frontend/`, served by `forge web serve`; no
  bundler, no new dependency.
- Target platform: Linux loopback server; the API runs in the same process
  family as today.

## 3. Ownership and shared code

- The Core functions keep their ownership: `doctor::run_doctor`,
  `governance::inspect`, `checker::build_document`, `profile::inspect_profile`.
  The status module is a thin read-only consumer; it **must not** reimplement
  detection, checking or readiness.
- `src/api/status.rs` owns the `forge-project-status/0.1.0` contract shape.
  It reuses `workbench::resolve` for id validation and unmanaged refusal so
  the "no path is ever supplied by the browser" rule has exactly one
  implementation, and `workbench::redact_local_paths` for any manifest-derived
  text it echoes.
- Path redaction reuses the existing `redact_local_paths` discipline; the
  status module never serializes `record.path` and strips the doctor report's
  `path` field if it ever renders a report fragment.
- No shared `/lib` or sibling extraction; project-local extension of existing
  owners only (per the proposal reconnaissance table).

## 4. Behavioral model

Actor: the single authenticated global admin (opaque `forge_admin_session`
cookie), already enforced by `guarded`.

### 4.1 Routes

| Route | Handler | Gate |
|---|---|---|
| `GET /v1/admin/projects/{id}/status` | `status::project_status(db_path, id)` | `guarded` (401 anon) + exact-origin CORS |
| `GET /v1/admin/status` | `status::fleet_status(db_path)` | `guarded` (401 anon) + exact-origin CORS |

Both are JSON-only reads. The 5-segment `{id}/status` arm cannot shadow the
existing 4-segment `{id}` detail arm, and the 3-segment `admin/status` arm
keeps any literal out of the `{id}` position. The existing
`OPTIONS ["v1","admin","projects",_,_]` and `OPTIONS ["v1","admin",_]` arms
already answer their preflight.

### 4.2 Project status response (`forge-project-status/0.1.0`)

```json
{
  "contract": "forge-project-status/0.1.0",
  "project_id": "demo",
  "management": "managed",
  "observed_at": "2026-10-07T00:00:00Z",
  "live": false,
  "state": "healthy",
  "note": "read-only in-process status; the DriftWatch policy adapter and native build/test toolchains are not run.",
  "checks": [
    { "id": "doctor",    "state": "healthy", "summary": "...", "reason": "" },
    { "id": "check",     "state": "healthy", "summary": "...", "reason": "" },
    { "id": "readiness", "state": "healthy", "summary": "...", "reason": "" }
  ]
}
```

`observed_at` is the registry observation timestamp (RFC 3339), labelled as
such. `reason` is a safe, path-free phrase; it is empty for `healthy`.

### 4.3 Sub-check mapping

Given the resolved `record` and `dir = Path::new(&record.path)`:

- **Unreadable root** (`!record.available || !dir.is_dir()`): all three
  sub-checks are `unavailable` with a single safe reason, overall
  `unavailable`; no Core call runs.
- **doctor**: `run_doctor(dir, None, Some(observation), None)`.
  - `Err` → `unavailable` ("the health inspection could not run; nothing was
    changed").
  - `Ok(report)`:
    - `healthy` true → `healthy`;
    - else if `report.stale` and no applicable `Fail` finding and no
      applicable unmet control → `stale`;
    - else → `issues`.
  - `summary` carries only counts (findings, unmet controls); no finding
    detail is echoed.
- **check**: only when `run_doctor` returned `Ok(report)` (otherwise
  `unavailable`).
  - `let governance = governance::inspect(dir);` (read-only evaluate).
  - `build_document(dir, &report, &governance, &checker::now_rfc3339(),
    checker::DEFAULT_MAX_ALERTS)`.
  - alerts empty → `healthy`; any `Error` alert → `issues`; only warning
    alerts → `issues`. `summary` carries the error/warning counts only.
- **readiness**: load the profile from the manifest
  (`Manifest::load_from_dir(dir, None)`).
  - manifest/profile absent → `unavailable` ("the manifest could not be read,
    so profile readiness cannot be evaluated").
  - `inspect_profile(profile)` `Ok(Supported)` → `healthy`.
  - `Ok(Planned)` → `issues` ("planned catalog candidate with no certified
    native template evidence").
  - `Err` → `issues` ("not a known catalog profile").
  - The echoed profile id passes through `redact_local_paths`.

### 4.4 Overall state reduction

Deterministic and documented: **`issues` > `unavailable` > `stale` >
`healthy`**. A real finding is never hidden behind missing evidence, and a
genuinely all-healthy project is `healthy`.

### 4.5 Fleet readiness summary

`GET /v1/admin/status` iterates `Registry::list()` (the registered projects),
computes each project's overall state with the same projection, and returns:

```json
{
  "contract": "forge-project-status/0.1.0",
  "live": false,
  "generated_at": "2026-10-07T00:00:00Z",
  "total": 3,
  "counts": { "healthy": 1, "issues": 1, "stale": 0, "unavailable": 1 },
  "projects": [ { "project_id": "demo", "state": "healthy" } ]
}
```

`projects` is bounded by `MAX_FLEET_STATUS_PROJECTS = 200`; when the registry
is larger the body carries `"truncated": true` and still counts every
registered project. The summary is non-live: it runs no adapter and no build.

### 4.6 Command catalog

- Add `ROUTE_PROJECT_STATUS = "GET /v1/admin/projects/{id}/status"` and
  `ROUTE_FLEET_STATUS = "GET /v1/admin/status"` to `IMPLEMENTED_WEB_ROUTES`.
- `check`: `cli_only` (`REASON_MACHINE_STDOUT`) → `web_at(None, "check",
  "Report the in-process doctor, governance and readiness status for one
  managed project (read-only).", Quality, Project, Read, ROUTE_PROJECT_STATUS,
  caps_web)`. No `execution` block (read-only, not a confirm-gated mutation).
- `fleet.status`: repoint from `WEB_ROUTE_PROJECTS` to `ROUTE_FLEET_STATUS`.
- Update the pinned web-row list in the in-source test and the catalog
  contract allowlist / `web_ids` set. The row count stays 227; `readiness.*`
  rows stay `cli_only` (`REASON_NATIVE`).

### 4.7 Frontend

- Workbench: add a "Project status" card (`#wb-status`) rendered from
  `GET /v1/admin/projects/{id}/status` after the detail load; overall state as
  a badge and each sub-check with its state and safe reason.
- Dashboard: add a fleet readiness tile near the summary grid, rendered from
  `GET /v1/admin/status`, showing the per-state counts and total. Both render
  with `textContent` only; no new HTML sink.

## 5. Failure and boundary behavior

| Condition | Result |
|---|---|
| No admin session | `401 api-unauthorized`, no status data |
| Non-configured origin | `403 admin-origin-rejected` |
| Hostile / path-bearing id | `400` typed refusal, no echo, no filesystem access |
| Valid but unmanaged / observed-only id | `404` typed refusal, no echo |
| Unreadable project root | overall + sub-checks `unavailable`, safe reasons |
| Real doctor/checker finding | `issues`, never a pass |
| Registry open failure | `503 admin-unavailable` |

## 6. Security / privacy

- No request may carry a path; the id is validated by
  `core::validate_project_id` and resolved by `registry.inspect`.
- No response serializes `record.path`, a credential, an adapter binary or a
  forbidden secret; manifest-derived/profile text passes `redact_local_paths`.
- The DriftWatch policy adapter is never invoked by the status surface, so no
  external process is started; the readiness matrix is never invoked, so no
  fixture is written.
- Existing `guarded` session gate, exact-origin CORS and JSON-only content
  type discipline are reused unchanged.

## 7. Test oracle

`tests/forge_web_project_status_contract.rs` drives the real `handle()` over a
throwaway registry and asserts:

1. Anonymous request to both routes → `401`, no data.
2. Valid-but-unmanaged id → `404` typed refusal, id not echoed.
3. Hostile/path-bearing id → `400`, offending input not echoed.
4. Healthy project (rust-web, L1, `Cargo.toml`, `README.md`, `Dockerfile`,
   `.github/workflows/ci.yml`, `driftwatch.yaml`, git repo with an `origin`
   remote + commit, freshly registered) → overall `healthy`, all three
   sub-checks `healthy`.
5. A project with a real missing build definition → doctor/check `issues` and
   overall `issues`, never `healthy`.
6. A registered project whose directory was removed → sub-checks
   `unavailable` with safe path-free reasons, overall `unavailable`.
7. No response body contains the registered absolute path or `"driftwatch"`/
   credential-shaped text; journal and project files are unchanged (proving
   read-only).
8. Fleet summary counts sum to `total == projects.len()` and the registered
   healthy project is counted `healthy`.
9. Catalog: `check` points at the project status route, `fleet.status` points
   at the fleet route, both `web` and both in `IMPLEMENTED_WEB_ROUTES`.
10. Frontend: `index.html` has the status card + fleet tile ids and `app.js`
    fetches both typed endpoints with no new unsafe sink.

## 8. Verification

- `cargo fmt` then `cargo fmt --check`.
- `cargo build` 0 errors.
- `cargo test` green for the new status contract plus
  `forge_web_command_catalog_contract` (updated), `forge_web_project_workbench_contract`,
  `forge_web_project_management_contract`, `forge_admin_api_contract`,
  `portal_ui_contract`, `catalog_contract`, `cargo test --lib api::` and
  `cargo test --bin forge`; record actual pass counts.
- `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive`.
- `git diff --check`; review newly added files.
