# Design: forge-web-gate-controls

## Ownership

The browser is a thin caller of the existing `forge::gate` Core path. The workbench's `Re-run gate` button issues `POST /v1/admin/projects/{id}/gate/run`, which is matched in `src/api/admin/gate.rs` (new) and registered as `Route::AdminProjectGateRun { id }` in `src/api/model.rs`. The handler in `src/api/admin/gate.rs` calls `forge::gate::run_gate` directly — never `cmd_gate`, never a subprocess of `forge` itself, never the CLI parser. The new `GET /v1/admin/projects/{id}/gate/status` matches the same module and calls `forge::gate::load_latest_evidence` + `forge::gate::evidence_freshness` + `forge::gate::evidence_summary` — the same three functions `cmd_gate_status` calls (`src/cli/functions_13.rs:381-438`). The gate runtime (Driftwatchdog / `driftwatch`) is external and untouched: the new handler inherits every existing posture, including `GateConfig::from_env()` (so `FORGE_GATE_BIN` is honoured), the bounded argument-array invocation, the `MAX_GATE_OUTPUT_BYTES` cap, the `capture_note` redaction and the `journal_verdict` mapping.

The runtime ownership boundary (`src/gate/mod.rs:1-37`) is preserved literally: the new handler never classifies a state the runtime did not report. `GateAggregate::Passed` is set only by `classify_aggregate` (`src/gate/engine.rs:489-502`) on a parseable document the runtime actually produced, with the `PASS` claim riding a successful exit; every other aggregate is whatever the runtime said (`Blocked`, `Failed`, `Unknown` for `PASS`-but-failed-exit / `REVIEW_REQUIRED` / unrecognised). Per-check states are whatever `GateCheckState::from_raw` mapped (`src/gate/model.rs:205-213`); an unknown raw state is `Unresolved`, never `Pass`.

The new `src/api/admin/gate.rs` is registered in `src/api/admin/mod.rs:30-37` `route_beside` so `router.rs` does not grow a single line of match arms — the precedent is `agent_identity::route_agent_identity` (`src/api/admin/agent_identity.rs:32-63`) which the router consumes via a single `.or_else` call. The two new `Route` variants are added in `src/api/model.rs` near the existing `AdminProjectHealthRefresh` (`:384-386`) so the model file is a single-place read.

## Contracts

### Wire shape

```
GET  /v1/admin/projects/{id}/gate/status       → 200 envelope
POST /v1/admin/projects/{id}/gate/run          → 200 envelope (or 409 / 502)
```

`{id}` is a validated project id (`forge::core::validate_project_id`); the browser never sends a path. The `resolved` arm of `src/api/workbench.rs:156-170` is the single resolver: validated id → `registry.inspect` → `ProjectRecord.path` → `&Path`. A cross-project or path-bearing id is a typed `400 admin-project-not-managed`.

`GET` request body: none. `POST` request body: empty body or `{}` (no fields are accepted; any non-empty body is a typed `415 admin-content-type-required`, mirroring `AdminProjectClassifyApply` at `src/api/admin/deploy.rs:104-120`).

### Success envelope (200, both routes)

```json
{
  "contract": "<GATE_CONTRACT_VERSION>",
  "project_id": "<id>",
  "gate": {
    "contract": "<GATE_CONTRACT_VERSION>",
    "project_id": "<id>",
    "runtime": "driftwatchdog",
    "runtime_version": "driftwatch 0.1.0",
    "revision": "<12-char sha or null>",
    "manifest_digest": "...",
    "rule_pack_version": "...",
    "aggregate": "passed|blocked|failed|unknown",
    "checks": [ { "id": "build", "state": "pass|fail|skip|not_applicable|unresolved", "note": "..." } ],
    "observed_at": "2026-10-14T00:00:00Z",
    "dry_run": false,
    "note": "...",
    "freshness": "fresh|stale|absent"
  },
  "persisted": "<relative path or null>",
  "journaled": true
}
```

`freshness` is computed against the project's current `git rev-parse HEAD` (the same call `cmd_gate_status` makes at `src/cli/functions_13.rs:403`) and overlays the record so a single wire surface always reports the same triple. The `gate` object's `aggregate` is exactly `evidence.aggregate.label()` and the `checks` are exactly the per-check rows the runtime mapped; the handler does not transform, redact-replace, or strip any check. The handler is responsible for **never** setting `gate.aggregate = "passed"` unless the runtime produced a parseable `PASS` document with exit success (the rule `classify_aggregate` already enforces); the wire is honest by construction.

### Absent evidence (200, both routes)

When `load_latest_evidence` returns `Ok(None)`, the wire is the same envelope with `gate = null`, `freshness = "absent"`, `aggregate = "unverified"`, and a `detail` field carrying the same note `cmd_gate_status` prints (`src/cli/functions_13.rs:394-397`): "the gate has never run; unverified is not a pass". The browser renders this as the same `unavailable` finding the doctor already renders; no claim of pass.

### Failure envelopes

| Condition | Status | `code` | Wire |
|---|---|---|---|
| Missing/expired admin session | 401 | `api-unauthorized` | standard session-gate response |
| Cross-origin request | 403 | `admin-origin-rejected` | standard CORS gate response |
| POST with non-JSON body | 415 | `admin-content-type-required` | `gate run requires application/json` |
| Unmanaged/unknown `{id}` | 400 | `admin-project-not-managed` | resolved by `src/api/workbench.rs:resolve` |
| Runtime answered a rehearsal on a real run | 409 | `gate-rehearsal-not-evidence` | `a rehearsal executes nothing; the cached evidence is untouched` |
| Runtime unavailable / timed out / unparseable | 502 | `gate-runtime-unavailable` | bounded reason from `GateOutcome::Unavailable` |
| Plan preview (dry-run) returned on real run | 502 | `gate-runtime-unavailable` | same envelope (a dry-run answer is never a verdict) |

The `502 gate-runtime-unavailable` wire preserves the bounded `reason` from `GateOutcome::Unavailable { reason }` so the operator sees the same diagnostic text the CLI prints (`capture_note` already redacts credentials and replaces the project path with `<project>`). The error envelope is the standard `{"error":{"code":"...","message":"..."},"contract":"<API_CONTRACT_VERSION>"}` shape; no project path leaks.

### Out of scope on the wire

- The raw sibling stdout/stderr (CLI's `render_evidence_human` does not project it on the JSON path; the wire stays at the `GateEvidence` envelope).
- The gate runtime's `--version` probe output beyond the bounded `runtime_version` field (one line, `bound_note` redacts).
- The sibling's blocking prediction / planned checks (those are the rehearsal surface and are not exposed on the real-run wire — a real run returns a `GateEvidence` with the runtime's actual checks, not the plan).

## Failures

The handler is built so that **every** failure mode renders the same honest state the doctor already renders for the same condition:

1. **Runtime unresolvable** (`FORGE_GATE_BIN` unset, no declaration, fallback probe finds no `driftwatchdog`/`driftwatch` on `PATH`). `run_gate` returns `GateOutcome::Unavailable { reason }`; the handler returns `502 gate-runtime-unavailable` with the bounded reason. The doctor renders the same condition as the `gate-evidence` finding's `unverified`/`unavailable` shape (`src/doctor/probes.rs:420-453`); the browser's `Gate status` sub-row shows the same `Unavailable` label.
2. **Runtime timeout** (`run_bounded` kills the child after the configured `Duration`). `run_gate` returns `Unavailable { reason: "timeout after …s (bounded wait; the gate never ran to completion)" }`; the handler maps it to `502 gate-runtime-unavailable` with the bounded reason. No aggregate is reported, no journal row is written, prior evidence stays byte-identical (`src/gate/mod.rs:33-36`).
3. **Runtime unparseable** (sibling emits non-JSON, or JSON without `blocked` + `results`, or unknown top-level status). `run_gate` returns `Unavailable { reason: "gate runtime `X` exited successfully without a parseable gate status document: …" }`; the handler maps it to `502 gate-runtime-unavailable`. Per the invariant: **no fabricated aggregate**, no `pass` from a missing document.
4. **Runtime answered a rehearsal on a real run** (`GateOutcome::Rehearsal {..}` on `dry_run: false`). The handler returns `409 gate-rehearsal-not-evidence`; prior evidence stays untouched; no journal row is written. The browser renders "the gate runtime answered a plan rehearsal instead of a verdict" in the `role=status` notice.
5. **Per-check `state` outside the known vocabulary** (e.g. a future sibling adds `PENDING_REVIEW`). `GateCheckState::from_raw` maps the unknown value to `Unresolved` and the check row's `note` carries the explanatory "state `X` is not in the known vocabulary and never counts as pass" prefix (`src/gate/engine.rs:441-451`); the wire preserves that `note` byte-for-byte. The aggregate is whatever `classify_aggregate` produced (`Unknown` for any non-`PASS`/`FAIL`/`REVIEW_REQUIRED` top-level status); never `Passed`.
6. **Stale evidence** (cached evidence exists, working tree revision moved). The handler overlays `freshness: "stale"` and the browser renders the same `Stale` label the doctor renders (`src/doctor/probes.rs:390-393`); no current-verification claim is made.
7. **Journal row failure** (`registry.record_operation` errors). The handler logs nothing and renders the same `502 gate-runtime-unavailable` envelope; the persisted evidence is still returned. The CLI is more strict here (`src/cli/functions_13.rs:669-675` — a journal failure does not block the run); the new handler mirrors that — a journal failure never causes the operator to lose the evidence they just produced.
8. **`validate_project_id` rejects `{id}`** (path separator, `..`, percent-encoded slash). The `resolve` helper returns `400 admin-project-not-managed`; this matches the existing `AdminProjectDetail` and `AdminProjectHealthRefresh` paths.
9. **Registry open failure / unwritable registry** (sandboxed environment). The handler returns `503 admin-registry-unavailable`; the prior evidence stays byte-identical; no journal row is written.

## Migrations

None. The change is additive:

- No new `forge::gate` API — the handler delegates to existing public functions (`run_gate`, `load_latest_evidence`, `evidence_freshness`, `evidence_summary`, `save_evidence`, `parse_timeout_secs`).
- No new CLI verb — the CLI is the source of truth and is unchanged.
- No new schema / migration — the journal row uses the existing `record_operation("gate", id, verdict, summary)` signature, the evidence file uses the existing `save_evidence` path, the contract version is the existing `GATE_CONTRACT_VERSION`.
- No new dependency — `serde_json`, `std::process` and `chrono` are already used by the existing handlers; the new module reuses the same `super::gateway::{cors, error, guarded, scrub_json, scrub_text}` envelope.
- No new external contact — the handler does not call any sibling URL; it either reads the persisted evidence (no subprocess) or spawns the gate binary the CLI already spawns.

The catalog `gate` row gains an `availability: "web"` cell for the run path and a sibling `gate.status` row is added with `availability: "web"`; the existing `REASON_TTY_HIDDEN` mapping on the run row is dropped (the run is no longer CLI-only; the rehearsal / `evidence` rows keep their `REASON_TTY_HIDDEN` because they remain CLI-only by design). One new `ROUTE_PROJECT_GATE_STATUS` and one new `ROUTE_PROJECT_GATE_RUN` constant in `src/api/admin/routes.rs` (mirroring `ROUTE_PROJECT_HEALTH_REFRESH` at `src/api/workbench.rs:54`); two new `WEB_ROUTE_ADMIN_*` constants and two new `IMPLEMENTED_WEB_ROUTES` entries in `src/api/command_catalog/routes.rs`.

Source-file-size posture:

- `src/api/router.rs` is at 1000/1000 lines. The change adds exactly one new match arm in the project-admin pre-table block (`src/api/router.rs:313-317` area, one new line), and two new arms in the existing `Route::AdminProject*` table sections (`:581` and `:770`/`:927` — each is a single `| Route::AdminProjectGateStatus { .. }` / `| Route::AdminProjectGateRun { .. }` line, ≤ 2 added lines per arm). Total router.rs growth: 4 lines (cap respected).
- `src/api/admin/gate.rs` (new) — mirror of `agent_identity.rs` (~80–120 lines), well below the 1000-line cap.
- `src/api/model.rs` — 2 new variants (~10 lines).
- `src/api/admin/mod.rs` — 1 new `pub mod gate;` + 1 new `or_else` (≤ 3 lines added).
- `frontend/index.html` — sub-row markup only, no new section.
- `frontend/app.js` — `loadGateStatus`, `renderGateStatus`, `runGateButton`, `runGate` (~50 lines) inside the existing workbench block; the token-pin test is the source-of-truth for the new `data-*` ids / `textContent` / `method: "POST"` discipline.

The workbench test pin (`tests/forge_web_project_workbench_contract.rs`) is updated to reflect the new sub-row; the workbench-health-latency pin is untouched (the new sub-row is additive inside the same `wb-health` box, the existing `policy-deferred` / `Run full health check` behaviour is unchanged).

The change is a sibling-operator-only surface: no provider, no adapter, no native toolchain, no network call, no `git push`, no `forge publish`, no `forge upgrade`, no `forge doctor` re-run. The browser mirrors the CLI; the CLI is the source of truth.
