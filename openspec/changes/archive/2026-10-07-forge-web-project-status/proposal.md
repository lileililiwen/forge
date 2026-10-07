# Proposal: Find the project status from the browser

## Why

The operator runs a 70+ project portfolio and reports they still cannot
*find the project status* from the portal:

- "i have 70+ project, and i publish a lot to the mac server"
- "i can still not manage project"
- "find the project status"
- "the project list is wrong"

The workbench (`forge-web-project-workbench`, archived) already surfaces a
single project's doctor **health** embedded in the detail projection, and the
command catalog (`forge-web-command-catalog`, archived) truthfully reports
which commands are browser-executable. But the portal has no dedicated,
read-only **status** view that answers "is this project healthy, and why not"
from the same doctor/checker/readiness evidence the CLI already computes, and
no fleet-wide readiness count. `forge check` is still catalogued `cli_only`
because it emits a machine-pure stdout document, so the browser cannot even
name it.

This package adds the read-only status surface the operator is asking for,
without forking any Core logic and without ever writing.

## What Changes

- Add a session-gated, read-only, JSON-only project status route:
  `GET /v1/admin/projects/{id}/status`. It reports an overall
  `state` (`healthy` / `issues` / `stale` / `unavailable`) plus three
  sub-checks, each with an honest state and a safe reason:
  - `doctor` — reuses `doctor::run_doctor` (policy adapter **not** run).
  - `check` — reuses `governance::inspect` + `checker::build_document` /
    `truncate_alerts`, the same in-process checker assembly the CLI uses.
  - `readiness` — reuses the read-only profile/readiness projection
    (`profile::inspect_profile`, `checker::readiness_alerts`); the native
    readiness matrix is never invoked.
- Add a session-gated, read-only fleet readiness summary:
  `GET /v1/admin/status`, counting the registered managed projects by overall
  status (`healthy` / `issues` / `stale` / `unavailable`) for a fleet tile.
- Recatalogue `check` from `cli_only` (`REASON_MACHINE_STDOUT`) to a `web`
  row pointing at the status route, and repoint `fleet status` from the
  projects-list route to the readiness summary route. Add both routes to
  `IMPLEMENTED_WEB_ROUTES`; row count stays 227.
- Render the per-project status card in the workbench and the fleet readiness
  tile in the dashboard, from the typed endpoints only (`textContent`, no new
  sink).
- No Core function changes; no new persistence; no shell, no argv, no
  browser-supplied path.

## Package Boundary and Split Assessment

The requested tier ("find the project status") is a single read-only outcome
with one owner and one oracle; it is **not** split. Its sibling follow-ons
already named by the previous tier are out of scope here:

| Package | Single outcome | Status |
|---|---|---|
| `forge-web-project-status` (**this**) | A signed-in operator reads one managed project's honest status and a fleet-wide readiness count, all read-only | this change |
| `forge-web-project-release` | Prepare/apply a release (git-network + subprocess stages) | follow-on, not touched |
| `forge-web-project-publish` | Drive provider publish / OpenPanel delivery | follow-on, not touched |

**Why this is the smallest independently verifiable unit:** every input is an
already-existing in-process function; the only new decision is which
read-only projection to call and how to reduce three sub-check states into one
honest overall state. There is no write, no network, no credential and no
subprocess boundary to design, so a single contract test over a throwaway
registry can pin the whole surface.

## Sibling and Shared Architecture Reconnaissance

This repository *is* the manager project; there is no sibling `common` /
`manager` to adopt from and no `/lib` extraction is in scope. The
reconnaissance is intra-repo: extend the existing owner of "read-only
single-project projection" rather than build a parallel mechanism.

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner | Decision |
|---|---|---|---|---|---|
| Session-gated admin gate | `src/api/admin.rs` `guarded`, `is_json`, `error`, `cors` | The exact 401/415/403 discipline every admin route uses | None | `src/api/admin.rs` | **reuse unchanged** |
| Single-project id resolution | `src/api/workbench.rs` `resolve`, `Resolved`, `redact_local_paths` | Validated opaque id + server-side root resolution + path redaction | Helpers are module-private | `src/api/workbench.rs` | **widen visibility** to `pub(super)` and reuse; do not duplicate the security-sensitive text |
| Doctor evidence | `src/doctor/mod.rs` `run_doctor`, `DoctorReport.findings`/`controls`/`stale`/`healthy` | The canonical health assessment | CLI `doctor` drives the DriftWatch adapter; the browser status must not | `src/doctor` | **reuse unchanged**; pass `policy_outcome = None` |
| Checker assembly | `src/main.rs` `cmd_check` + `src/checker/mod.rs` `build_document`, `truncate_alerts`, `readiness_alerts`, `doctor_alerts`, `governance_alerts` | The exact in-process checker plane | `cmd_check` owns the CLI-only stdout framing | `src/checker` | **reuse unchanged**; assemble the document in-process and project alert counts |
| Governance observation | `src/governance.rs` `inspect` = `evaluate_project` | Read-only evaluation, no persisted observation | `check_project` persists; not used | `src/governance` | **reuse unchanged** (`inspect`, not `check_project`) |
| Readiness projection | `src/readiness/mod.rs` `matrix_profile_ids`; `src/profile/mod.rs` `inspect_profile`, `ProfileSupportStatus` | Supported vs planned catalog state | `run_matrix`/`run_profile_row` write fixtures and run subprocesses | `src/readiness` | **reuse the read-only projection only**; never call the matrix |
| Command catalog | `src/api/command_catalog.rs` `IMPLEMENTED_WEB_ROUTES`, `web_at`, `web`, pinned web-list tests | The single source of truth for browser availability | `check` row is `cli_only`; `fleet.status` points at the list route | `src/api/command_catalog.rs` | **extend shared owner** |
| Frontend | `frontend/app.js` `renderHealth`, `dashboardPage`, `loadWorkbenchDetail` | Existing typed-endpoint render + honest-state pattern | No status card / fleet tile | `frontend/app.js` + `frontend/index.html` | **extend owner** |

No shared extraction or sibling edit is authorized; every decision is an
in-repo extension of an existing owner.

## BFS Impact Map

- **Capabilities:** `forge-web-project-status` (new);
  `forge-web-command-catalog` (modified: `check` and `fleet status` become
  `web` with the real read-only routes).
- **Users / flows:** the single global admin operator, on the dashboard and
  inside a project's workbench.
- **Contracts / data / persistence:** no schema change, no write, no journal
  row. Two new read-only JSON response shapes (`forge-project-status/0.1.0`).
- **Integrations / configuration:** none. No provider is contacted; the
  DriftWatch adapter is not run.
- **Callers:** `src/api/mod.rs` router/group/dispatch; `src/api/admin.rs`
  handle arms; `src/api/status.rs` (new); `src/api/workbench.rs` (visibility
  only); `src/api/command_catalog.rs` (rows + allowlist + tests);
  `frontend/app.js` + `frontend/index.html`; the new contract test.
- **Failure / boundary behavior:** anonymous → `401`; hostile/path-bearing id
  → `400` no echo; unmanaged/observed-only id → `404` no echo; unreadable
  project root → sub-checks `unavailable` with safe reasons; a real doctor or
  checker finding → `issues`, never a pass.
- **Tests:** new `tests/forge_web_project_status_contract.rs`; the catalog
  contract strict allowlist and `web_ids` set must be updated; the workbench
  and other `forge_web_*` contracts must stay green.
- **Dependencies / compatibility:** depends on the archived
  `forge-web-project-workbench` and `forge-web-command-catalog` shapes; no
  breaking change to any bearer `/v1` route or the workbench projection.
- **Privacy / security:** the browser supplies a validated id only; no
  response serializes an absolute path, credential or adapter binary; no
  shell, argv or path is ever interpreted.

## Capabilities

- `forge-web-project-status` — new: read-only single-project status
  (doctor/check/readiness) and a fleet readiness summary, on the
  session-gated admin surface.
- `forge-web-command-catalog` — modified: `check` and `fleet status` report
  their real read-only web routes and their CLI-only next steps are gone.

## Non-goals

- No write of any kind: no registry row, no journal entry, no project file,
  no fixture, no native build/test.
- No browser execution of `gate`, `test`, readiness `matrix`/`check`,
  release, publish, provider or delivery commands.
- No DriftWatch policy-adapter invocation and no external provider probe.
- No new route that accepts a filesystem path, binary, host, argv or shell
  string; the project is always resolved from a validated id.
- No change to `API_CONTRACT_VERSION` or any bearer `/v1` route.
