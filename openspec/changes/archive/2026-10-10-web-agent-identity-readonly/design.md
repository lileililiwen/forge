# Design: web-agent-identity-readonly

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate
  (library) plus `frontend/`. No sibling touched.
- **Modules changed:**
  - New `src/api/admin/agent_identity.rs` — `route_agent_identity`
    matcher plus five session-gated GET handlers reusing
    `deploy_id_gate` (hostile id → `400
    admin-invalid-project-id` without echo, unmanaged id → `404
    admin-project-unmanaged`), `guarded` (global admin session
    gate) + `cors` + `error`/`scrub_*` from `gateway.rs`. Kept
    beside the handlers (not in `router.rs`) so the route table
    stays under the source-file-size cap — the `history.rs`
    precedent exactly.
  - `src/api/admin/routes.rs` — five route constants:
    `ROUTE_ADMIN_AGENT_LIST`
    (`GET /v1/admin/projects/{id}/agents`),
    `ROUTE_ADMIN_AGENT_STATUS`
    (`GET /v1/admin/projects/{id}/agents/{session_id}`),
    `ROUTE_ADMIN_IDENTITY_CONFIG`
    (`GET /v1/admin/projects/{id}/identity/config`),
    `ROUTE_ADMIN_IDENTITY_SESSIONS`
    (`GET /v1/admin/projects/{id}/identity/sessions`),
    `ROUTE_ADMIN_IDENTITY_SESSION_INSPECT`
    (`GET /v1/admin/projects/{id}/identity/sessions/{session_id}`).
    The `agents`/`identity` literals never collide with the
    `releases`/`deploys`/`delivery`/`catalog` arms, so trying the
    matcher first shadows no existing route.
  - `src/api/model.rs` — three `Route` variants with doc comments:
    `AdminProjectAgents { id, session: Option<String> }`,
    `AdminProjectIdentitySessions { id, session: Option<String> }`,
    `AdminProjectIdentityConfig { id }`. List vs inspect folds on
    `Option` so the three exhaustive `router.rs` matches grow by 3
    arms each (not 5).
  - `src/api/router.rs` — compact threading only (≈ +8 lines
    against the 991/1000 cap, verified with `wc -l` before
    commit): extend the history pre-check comment + chain
    `route_agent_identity` via `or_else`; 3 arms in
    `required_permission` (all `None` — the `admin::handle`
    `guarded` gate does the real check); 3 arms in the
    `admin::handle` short-circuit; 3 arms in the trailing
    exhaustiveness `not_found` list; widen the six-segment
    OPTIONS arm in place to `["v1", "admin", "projects", _, _, _,
    ..]` so the 7-segment identity inspect preflights without
    adding a line.
  - `src/api/admin/deploy.rs` — one combined dispatch arm routing
    all three variants to `agent_identity::handle` (same shape as
    the existing per-route `guarded` arms).
  - `src/api/command_catalog/routes.rs` — five
    `WEB_ROUTE_ADMIN_*` consts + `IMPLEMENTED_WEB_ROUTES` entries.
  - `src/api/command_catalog/rows_platform.rs` — five leaves
    `NotYetWeb` → `web_at` (all `Read`); `identity.session-terminate`
    leaf → `cli_only` with `REASON_LOCAL_FS` (deletes persisted
    session state); `identity.build-challenge` untouched
    (`NotYetWeb` — persist + `code_verifier` secret, §6).
  - `frontend/index.html` — one "Agent & identity" `wb-card` in
    the Workbench grid reusing `#workbench-project`.
  - `frontend/app.js` — read-only loaders/renderers reusing
    `request`, `el`, `detailRow`, `renderErrorSummary`,
    `setResultRole` (the history-card read-only pattern).
  - `tests/web_agent_identity_readonly_contract.rs` — new contract
    binary (§8 oracle).

## 2. Ownership and contracts

- **Owner of truth:** Core owns the persisted shapes
  (`AgentSession`, `SessionListEntry`, `AdminSession`,
  `IdentityConfig`); the handlers project them to path-free JSON
  and never invent fields.
- **Path scrub:** `AgentSession.project_path` is absolute and MUST
  be projected out of the status view (return `session_id`,
  `project_id`, `provider`, `spec_id`, `state`, `started_at`,
  `last_transition_at`, `transitions`, `backing` only).
  `SessionListEntry` and `AdminSession` carry no path. Error
  strings are scrubbed of the project dir via `scrub_text`. The
  contract test asserts the raw body never contains the project
  dir string.
- **No secret material:** no new route builds an `AuthChallenge`
  (so no `code_verifier` exists to leak), reads no password, and
  returns no credential value. `IdentityConfig.client_secret_ref`
  is echoed verbatim (it is a `env://NAME` reference, never a raw
  secret — the validator refuses raw secrets in the manifest).
  `AdminSession.session_id` values are returned: they are the
  identifiers the operator needs to inspect, visible only behind
  the global admin session gate (same posture as the CLI, which
  prints them).
- **Inspect-id validation:** `{session_id}` segments are validated
  before any filesystem read. Agent ids use the kebab-case session
  rules (empty, `/`- or `\`-bearing, `..`-bearing or
  `%`-bearing → typed `400 admin-invalid-agent-session-id`
  without echo). Identity ids must be non-empty hex tokens
  (the `session_path_for` invariant; anything else → typed `400
  admin-invalid-identity-session-id` without echo). Unknown but
  well-formed ids → typed `404`; a hex id owned by a sibling
  project → typed `403` cross-project refusal via
  `lookup_session_in_sibling_projects` (the CLI `session-inspect`
  boundary, mapped through `err_status`).
- **No live probe:** the agent status handler calls `status_for`
  (`read_session`) only — never `live_runtime_status` (which
  spawns `ariadex status --json`). Supervised sessions return
  `live: null` plus `live_note: "live runtime state is a CLI
  observation (`forge agent status`); the browser renders the
  recorded session file."` Adapter availability uses the
  spawn-free `probe_provider` PATH check only, surfaced as
  `adapter: { available, reason }` — an absent binary renders
  honest unavailable-with-reason, never a fabricated state.
- **Identity config read:** `Manifest::load_from_dir` +
  `IdentityConfig::from_manifest_opt`. `Ok(None)` (no `identity:`
  block) → typed `404 admin-identity-unconfigured`; `Err` →
  the typed `identity-invalid` code through `err_status` (a
  misconfigured block reports honestly instead of reading as
  absent). Nothing is persisted, no challenge is built, no
  provider is contacted.

## 3. Frontend contract

- One `wb-card` ("Agent & identity") inside `#workbench-body
  .workbench-grid`, reusing `#workbench-project` — the same
  missing-project refusal (error summary + focus) as the history
  card.
- Controls (all native buttons/inputs, no new dependency, no
  `innerHTML`, no inline script): List agent sessions / agent
  session inspect (id input); List identity sessions / identity
  session inspect (id input); Read identity config. Each result
  renders into its own `dl[role=status]` via `detailRow` scalars;
  failures report through the workbench error summary with focus.
- Explicit reads only — no auto-load on workbench open (no new
  latency on the open path, no provider/adapter touched by a
  view render).

## 4. Catalog wiring

- `agent.status` → `web_at(ROUTE_ADMIN_AGENT_STATUS)`,
  `agent.list` → `web_at(ROUTE_ADMIN_AGENT_LIST)`,
  `identity.validate-config` →
  `web_at(ROUTE_ADMIN_IDENTITY_CONFIG)`, `identity.session-list` →
  `web_at(ROUTE_ADMIN_IDENTITY_SESSIONS)`,
  `identity.session-inspect` →
  `web_at(ROUTE_ADMIN_IDENTITY_SESSION_INSPECT)` — all `Read`.
- `identity.session-terminate` → `cli_only(REASON_LOCAL_FS)`.
- `IMPLEMENTED_WEB_ROUTES` +5; catalog count pin stays 234;
  `identity.build-challenge` stays `NotYetWeb`.
- Existing pins to update alongside: `catalog.rs` web-id vec,
  `tests/forge_web_command_catalog_contract.rs` web-route + web-id
  allowlists, lib `web_rows_only_point_at_implemented_routes`.

## 5. Failure boundaries

- Anonymous → `401` before any Core call (`guarded`).
- Hostile project id → `400` without echo; unmanaged id →
  `404`; bad session id → `400` without echo; unknown session →
  `404`; sibling-owned identity session → `403`; unconfigured
  identity block → `404`; invalid identity block → typed
  `identity-invalid` via `err_status`.
- Refused reads create no `.forge/agents` / `.forge/identity`
  entries, no challenge files and no journal rows (asserted by
  the contract test via before/after directory + journal
  comparison).
- Unavailable adapter (binary absent) → `200` with the recorded
  session plus `adapter.available: false` and the probe reason —
  never a fabricated live state.

## 6. Why build-challenge stays out

`forge identity build-challenge` (a) writes a pending challenge
file (`save_challenge` — a mutation, so a GET must not run it)
and (b) returns `code_verifier`, the one secret the provider never
sees. A read-only browser preview would either persist orphans
(`complete-auth` stays CLI-only, so nothing could consume them)
or leak the verifier. It stays `NotYetWeb` with its existing
reason until and unless a persist-free, verifier-free preview is
designed — explicitly out of scope here.

## 7. What stays CLI-only (non-goals, enforced by no route)

`agent start|pause|takeover|resume|restart|new-session|run-spec`
(`REASON_AGENT`), `identity
setup|change-password|generate-password` (TTY/secret reasons),
`identity complete-auth|session-validate` (already `cli_only`),
`identity session-terminate` (newly `cli_only`), `identity status`
(already `cli_only`). The contract test asserts none of these ids
is `web` and no new POST route exists for them.

## 8. Test oracle (contract binary)

Auth (anon `401` on all five); hostile id `400` without echo;
unmanaged id `404`; agent list empty → `200` empty, seeded
session file → entry; agent status found (path-free, `live:
null`, adapter availability) / unknown `404` / hostile `400`;
identity sessions empty + seeded list; identity inspect found /
unknown `404` / non-hex `400` / sibling-owned `403`;
config validated (`200`, provider/issuer/client_id shapes, no
provider contacted) / unconfigured `404` / invalid block typed
`identity-invalid`; raw bodies never contain the project dir;
refused reads change no state dirs or journal; catalog pins (5
`web_at`, terminate `cli_only`, build-challenge still
`not_yet_web`, count 234); `IMPLEMENTED_WEB_ROUTES` contains the
5; frontend pins (card + control ids, no `innerHTML` in added
code, no new dependency).
