# web-agent-identity-readonly Specification

## Purpose
Let dashboard operators read per-project agent and identity session state without the CLI: five read-only admin GETs over the existing Core agent (`.forge/agents/`) and identity (`.forge/identity/`) stores plus manifest identity-block validation (no writes, no provider, no adapter subprocess, no secret material), with the five catalog leaves converted to `web` and a read-only Workbench card.
## Requirements
### Requirement: Read-only agent sessions in the browser

Forge SHALL serve the per-project recorded agent session list and
single-session status through session-gated admin GET routes reusing
`agent::list_sessions` and `agent::status_for`. Reads SHALL never
write state, spawn an adapter subprocess, contact a provider, or
serialize the absolute project path.

#### Scenario: Agent list reads back recorded sessions

- **WHEN** an operator GETs
  `/v1/admin/projects/{id}/agents` for a project with recorded
  agent sessions
- **THEN** the response carries one entry per recorded session
  (`session_id`, `provider`, `state`, `spec_id`,
  `transition_count`) and a project with no sessions gets `200`
  with an empty list

#### Scenario: Agent status resolves or 404s honestly without probing

- **WHEN** an operator GETs
  `/v1/admin/projects/{id}/agents/{session_id}`
- **THEN** a recorded session returns its stored state with the
  project path projected out, `live: null` with a CLI-only note,
  and spawn-free adapter availability; an unknown id returns typed
  `404` with nothing written and no subprocess spawned

### Requirement: Read-only identity sessions in the browser

Forge SHALL serve the per-project persisted identity session list
and single-session inspect through session-gated admin GET routes
reusing `identity::list_sessions` and `identity::load_session`.
Reads SHALL never write state, build a challenge, mint or revoke a
session, or return secret material.

#### Scenario: Identity session list and inspect stay scoped

- **WHEN** an operator GETs
  `/v1/admin/projects/{id}/identity/sessions` and
  `/v1/admin/projects/{id}/identity/sessions/{session_id}`
- **THEN** entries carry the persisted session fields with no
  credential values, an unknown id returns typed `404`, and a
  hex id owned by a sibling project returns a typed `403`
  cross-project refusal with nothing written

### Requirement: Read-only identity config validation in the browser

Forge SHALL serve `GET
/v1/admin/projects/{id}/identity/config` from the manifest's
`identity:` block via `IdentityConfig::from_manifest_opt` without
contacting any provider, building any challenge, or persisting
anything.

#### Scenario: Config read validates or reports honestly

- **WHEN** an operator GETs the identity config route
- **THEN** a well-formed block returns its validated shape, a
  project with no `identity:` block gets typed `404`, and an
  invalid block reports its typed `identity-invalid` reason —
  with no provider contacted on any path

### Requirement: Agent/identity routes share the admin read boundary

All five agent/identity routes SHALL refuse anonymous requests with
`401` before any Core call, refuse hostile project ids with typed
`400` without echo, refuse unmanaged ids with typed `404`, refuse
bad session ids with typed `400` without echo, and SHALL never
create `.forge/agents` / `.forge/identity` entries, challenge
files or journal rows on any refused or read path.

#### Scenario: Refused reads change nothing

- **WHEN** an operator hits any agent/identity route anonymously,
  with a hostile or unmanaged id, or with a bad session id
- **THEN** Forge answers the typed refusal and the state dirs and
  journal are identical before and after

### Requirement: Catalog names the agent/identity reads as web

The five leaves `agent.status`, `agent.list`,
`identity.validate-config`, `identity.session-list` and
`identity.session-inspect` SHALL convert `NotYetWeb` → `web`
pointing at the exact admin routes the router registers (row count
stays 234); `identity.session-terminate` SHALL be `cli_only`
(a write the browser must never trigger) and every lifecycle,
secret and transport verb SHALL stay `cli_only`, with
`identity.build-challenge` remaining `not_yet_web` (persist +
`code_verifier` secret boundary).

#### Scenario: Catalog rows resolve to implemented routes

- **WHEN** the catalog is rendered
- **THEN** the five read leaves carry `web` with routes from
  `IMPLEMENTED_WEB_ROUTES`, the count stays 234, and no
  lifecycle/secret/transport leaf is `web`

### Requirement: Read-only agent/identity card in the Workbench

The Workbench view SHALL offer a read-only "Agent & identity" card
reusing the existing project select: agent list / inspect,
identity session list / inspect and identity config read over the
five routes above, with `role=status` results, error-summary focus
and native controls. The card SHALL issue no write, run no
adapter, contact no provider and introduce no new frontend
dependency.

#### Scenario: Operator reads sessions without the CLI

- **WHEN** an operator opens a managed project in the Workbench
  and uses the Agent & identity card
- **THEN** lists, inspects and the config read render recorded
  state (or honest empty/unavailable-with-reason), failures focus
  the error summary, and no new JS errors attributable to the
  card appear
