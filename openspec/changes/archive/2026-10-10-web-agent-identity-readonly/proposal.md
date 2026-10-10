# Proposal: web-agent-identity-readonly

## Why

The dashboard operator can plan/apply deploys, releases, publishes and
browse their history from the browser, but has zero visibility into the
two session surfaces every project accumulates: `agent status|list`
(recorded agent sessions under `.forge/agents/`) and `identity
session-list|session-inspect` (persisted OIDC admin sessions under
`.forge/identity/`), plus the `identity validate-config` read that
proves the manifest's `identity:` block is well-formed without
contacting any provider. All six reads (`agent.status`, `agent.list`,
`identity.validate-config`, `identity.build-challenge`,
`identity.session-list`, `identity.session-inspect`) are `not_yet_web`
in the command catalog. Verifying "which agent session is bound to
this project?" or "which admin sessions exist for this project?" from
the dashboard today means dropping to a terminal.

## What Changes

- **Five read-only typed admin GET routes** (session-gated, JSON-only,
  no write, no provider, no adapter subprocess, no shell, no
  browser-supplied path), matched beside their handlers in a new
  `src/api/admin/agent_identity.rs` (the `history.rs` precedent, so
  `router.rs` stays under the source-file-size cap):
  - `GET /v1/admin/projects/{id}/agents` — agent session list via
    `agent::list_sessions` (recorded entries only).
  - `GET /v1/admin/projects/{id}/agents/{session_id}` — one recorded
    agent session via `agent::status_for` (`read_session`); unknown
    id → typed `404`. Never calls `live_runtime_status` (that spawns
    `ariadex status`); supervised sessions get `live: null` plus an
    honest CLI-only note. Adapter availability is reported through
    the spawn-free `probe_provider` PATH check only.
  - `GET /v1/admin/projects/{id}/identity/config` — manifest
    identity-block validation via `IdentityConfig::from_manifest_opt`
    (no provider contact, no challenge built, nothing persisted);
    project with no `identity:` block → typed `404`.
  - `GET /v1/admin/projects/{id}/identity/sessions` — persisted admin
    sessions via `identity::list_sessions` (stable id order, as the
    CLI reads them).
  - `GET /v1/admin/projects/{id}/identity/sessions/{session_id}` —
    one persisted session via `identity::load_session`; unknown id →
    typed `404`; id owned by a sibling project → typed `403`
    cross-project refusal (the CLI's `session-inspect` boundary).
- **Route threading:** three `Route` variants
  (`AdminProjectAgents { id, session: Option<String> }`,
  `AdminProjectIdentitySessions { id, session: Option<String> }`,
  `AdminProjectIdentityConfig { id }`; `None` = list, `Some` =
  inspect — one variant per shape family so the three exhaustive
  `router.rs` matches grow by 3 arms each, keeping `router.rs` under
  the 1000-line cap), five `ROUTE_ADMIN_*` constants, five
  `IMPLEMENTED_WEB_ROUTES` entries, and a 6+-segment OPTIONS arm
  widened in place (covers the 7-segment identity inspect without
  adding a line).
- **Catalog:** five leaves `NotYetWeb` → `web_at`
  (`agent.status`, `agent.list`, `identity.validate-config`,
  `identity.session-list`, `identity.session-inspect`); row count
  stays 234 (conversion, not addition).
  `identity.session-terminate` converts `NotYetWeb` leaf → `cli_only`
  (it deletes persisted session state — a write the browser must
  never trigger). `identity.build-challenge` stays `NotYetWeb`: it
  persists a pending challenge file (a write) and returns the
  `code_verifier` secret, which must never cross the JSON API; the
  OIDC round-trip is a non-goal.
- **Frontend:** the Workbench grid gains one read-only
  "Agent & identity" card reusing the existing `#workbench-project`
  select (no new select, no new dependency): agent list / inspect,
  identity sessions list / inspect, identity config read;
  `role=status` results, error-summary focus, keyboard-operable
  native controls. No auto-load on workbench open (explicit reads
  only, like the Delivery history card).
- **Contract tests:** new
  `tests/web_agent_identity_readonly_contract.rs` pins auth, id
  gates, empty/non-empty lists, inspect found/missing/cross-project,
  config validated/unconfigured, path-leak scrub, no-write-on-read,
  catalog pins and frontend token pins.

## BFS Impact Map

- **Requirements/scenarios:** `agent-runtime-workflows` (agent
  status/list reads), `central-admin-identity` (identity session
  list/inspect + validate-config reads), `portal-web-ui` (Workbench
  card). No requirement text changes — this closes the `not_yet_web`
  gap those specs already track via the catalog.
- **Concepts/modules:** `agent::{list_sessions, status_for,
  probe_provider}`, `identity::{list_sessions, load_session,
  lookup_session_in_sibling_projects, IdentityConfig}`,
  `core::manifest::Manifest`; new `src/api/admin/agent_identity.rs`;
  `Route` enum + router arms + `admin/deploy.rs` dispatch; catalog
  `rows_platform.rs` + `routes.rs`; Workbench card in
  `frontend/index.html` + `frontend/app.js`.
- **Contracts:** five new `ROUTE_ADMIN_*` constants;
  `IMPLEMENTED_WEB_ROUTES` +5; catalog count pin stays 234;
  `API_CONTRACT_VERSION` unchanged; every response carries
  `contract: API_CONTRACT_VERSION`.
- **Callers/persistence:** no Core/CLI/registry-schema change; reads
  open the registry read-only path and the project's
  `.forge/agents/` / `.forge/identity/` dirs; refused reads create
  no session/challenge files and no journal rows.
- **Integrations:** no provider, no adapter subprocess, no git write,
  no native toolchain, no network on any path; `build-challenge`
  (persist + secret) and `complete-auth` stay CLI-only.
- **Tests/compatibility:** new contract binary + regression over
  `forge_web_command_catalog_contract`, `portal_ui_contract`,
  `forge_web_project_workbench_contract`; old-frontend safety:
  unknown ids stay typed refusals.
- **Concerns:** security (id gates, no echo, path scrub, 401 before
  Core, no secret material — `code_verifier` never built, passwords
  never read, `project_path` projected out); quality (no
  placeholder, no new dep, no `innerHTML`, file-size caps);
  a11y (live regions, focus, native controls).
- **Verification:** fmt, build, targeted + regression suites, names
  preflight, strict validate, governance, diff-check, live
  throwaway-registry zero-JS-error proof, gate dry-run + bounded
  full gate.

## Capabilities

- Authenticated dashboard operator with a managed project.
- Read-only browser visibility into that project's recorded agent
  sessions, persisted identity sessions and validated identity
  configuration.

## Non-goals

- Any agent lifecycle write (`start|pause|takeover|resume|restart|
  new-session|run-spec`): adapter/TTY boundary, stays CLI-only.
- Any identity secret write or revocation (`setup|change-password|
  generate-password|status`, `complete-auth|session-validate|
  terminate`): TTY/secret/mutation boundary, stays CLI-only.
- OIDC round-trip in the browser (`build-challenge` persist +
  `code_verifier`, callback completion): transport + secret
  boundary, stays CLI-only.
- Transports (`api serve`, `web serve`, `mcp`, `portal view`):
  long-running/legacy surfaces, untouched.
- Live adapter probing (`ariadex status` subprocess, PTY manager):
  no subprocess is spawned from any new route; liveness stays a
  CLI observation.
