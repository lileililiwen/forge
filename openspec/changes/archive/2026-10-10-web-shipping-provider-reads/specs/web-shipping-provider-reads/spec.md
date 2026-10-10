# web-shipping-provider-reads (delta)

## ADDED Requirements

### Requirement: Typed read-only shipping provider routes

Forge SHALL serve five typed read-only session-gated admin GETs reusing
the existing Core stores with no new filtering, ordering or pagination
rule in any transport: `GET /v1/admin/projects/{id}/publish/providers`
(list) and `GET .../publish/providers/{provider}` (inspect) over the
server-resolved `.forge/providers.yaml`; `GET /v1/admin/providers/matrix`
(matrix, never live) and `GET /v1/admin/providers/{provider}` (static
inspect) over the evidence-provider table; and `GET
/v1/admin/projects/{id}/plugins` (list) over the same config plus its
sibling `plugins:` block. Every route SHALL be exact-origin checked,
registry read-only, journal-free, probe-free, adapter-free, shell-free,
network-free and browser-path-free, and SHALL carry both the registry
contract and the admin contract versions.

#### Scenario: Publish provider lists agree with the CLI

- **WHEN** an operator GETs `/v1/admin/projects/alpha/publish/providers`
- **THEN** the entries are byte-identical to `forge publish provider list
  --format json` payloads for the same server-resolved config in the same
  stable order, with absolute paths scrubbed; a missing config answers
  honest `unavailable-with-reason`, never a 500

#### Scenario: Publish provider inspects agree with the CLI

- **WHEN** an operator GETs
  `/v1/admin/projects/alpha/publish/providers/<provider>`
- **THEN** the entry is byte-identical to `forge publish provider inspect
  <provider> --format json`; an unknown provider answers typed
  `unknown-provider` and a hostile segment answers a static typed `400`
  without echo

#### Scenario: Evidence matrix never probes and never claims support

- **WHEN** an operator GETs `/v1/admin/providers/matrix`
- **THEN** the matrix is byte-identical to `provider::matrix(false)`:
  every row is `not-run`, `live` is false, no binary is probed, no
  journal row is written, and no row ever claims support

#### Scenario: Evidence provider inspects never probe

- **WHEN** an operator GETs `/v1/admin/providers/<provider>`
- **THEN** the descriptor is byte-identical to `provider::inspect`
  without probing or contacting anything; an unknown provider answers
  the Core typed error and a hostile segment answers a static typed
  `400` without echo

#### Scenario: Plugin lists agree with the CLI

- **WHEN** an operator GETs `/v1/admin/projects/alpha/plugins`
- **THEN** the records are byte-identical to `forge plugins list
  --format json` payloads for the same server-resolved config in the
  same stable order, with absolute paths scrubbed; a missing config
  answers an honest empty registry (`plugins: []`), mirroring the CLI

#### Scenario: Reads write nothing and probe nothing

- **WHEN** an operator runs the five reads in any order
- **THEN** no file, registry byte or journal row is written, no provider
  binary or adapter runs, no network is contacted, and the registry
  bytes are identical before and after the reads

### Requirement: Read-only shipping panel in the delivery view

Forge SHALL render one compact read-only "Provider & plugin reads" card
in the `#delivery` view (after the history card, before confirmed
actions) that drives the five routes above through explicit clicks only,
renders honest unavailable-with-reason states, and lists the excluded
probe/write verbs (`publish sync|db|fleet|prepare|deploy|all`, `docs
translate`, `provider run`, `fleet online`, `project.github *`,
`push`/`mirror`, `readiness *`, `deploy observe`, transports,
agent/identity lifecycle writes, `publish provider enable|disable`) as
availability badges with reasons plus a pointer to the existing
fleet-inspect control. The card SHALL add no frontend dependency and use
explicit fetches only.

#### Scenario: Operator browses shipping reads without the terminal

- **WHEN** a signed-in operator opens the delivery view and clicks the
  five loaders
- **THEN** each result matches the corresponding CLI JSON payload (paths
  scrubbed), unavailable configs render with their reason, excluded
  verbs render as badges with reasons, and zero JS console/page/network
  errors occur
