# forge-web-workspace-onboarding Specification

## Purpose
TBD - created by archiving change forge-web-workspace-onboarding. Update Purpose after archive.
## Requirements
### Requirement: Live candidate discovery without host coupling

Forge SHALL expose, on the session-gated global admin surface, a read-only
route that re-reads the operator-configured project root on every request
and returns a bounded, path-free candidate view of its immediate child
directories. Each candidate SHALL carry its leaf name, derived id, manifest
presence, suggested profile/confidence when decidable, live registration
state and next action. The response SHALL contain no absolute path, manifest
body, credential or secret, and no concrete host folder SHALL appear in code,
specs or responses — the root is runtime-only operator configuration.

#### Scenario: Signed-in operator discovers candidates

- **WHEN** an authenticated admin requests candidates with a configured root
- **THEN** Forge returns one entry per immediate child directory with its live state, invokes no provider and writes nothing

#### Scenario: Workspace grows between requests

- **WHEN** a new sibling directory appears after a first discovery
- **THEN** the next discovery response includes it with no code, list or configuration change

#### Scenario: Root is unconfigured

- **WHEN** `FORGE_ADMIN_PROJECTS_ROOT` is unset, blank or not a directory
- **THEN** Forge refuses with typed `409 admin-prerequisite` naming the variable and reads nothing

#### Scenario: Hostile leaf or hostile input

- **WHEN** a caller supplies a path separator, traversal segment, NUL byte, control character, `.` or `..`
- **THEN** Forge refuses with a typed `400` that echoes nothing and performs no filesystem read

### Requirement: Bulk preview, confirmation and honest per-item results

Forge SHALL require an explicit item selection plus `confirm: true` and a
`plan_digest` matching the exact canonical selection before onboarding
anything. An unconfirmed request SHALL return per-item previews and a digest
and write nothing. A mismatched digest SHALL be refused with a fresh preview
and write nothing. A confirmed matching request SHALL import or register each
item through the same Core functions the CLI runs and SHALL report honest
per-item success/failure; any failure SHALL surface as a partial result,
never a blanket success.

#### Scenario: Selection preview writes nothing

- **WHEN** a signed-in operator submits a candidate selection without `confirm: true`
- **THEN** Forge returns per-item plans and a 64-hex digest and changes no directory, manifest or registry row

#### Scenario: Changed set is refused

- **WHEN** the directory set changes between preview and confirm
- **THEN** the digest no longer matches, Forge refuses with `409 admin-digest-mismatch` and writes nothing

#### Scenario: Confirmed batch applies per item

- **WHEN** the operator confirms with the matching digest
- **THEN** Forge adopts or registers each candidate, journals each outcome, appends one counts-only parent row, and returns 202 when all succeed or 207 with per-item codes when any fail

### Requirement: Task-oriented onboarding interface

Forge SHALL present workspace onboarding as a discover → review → select →
preview → confirm → results journey in the dashboard management area. The
panel SHALL show candidate leaf, derived id, profile signal, registration
state and next action; allow per-item profile/id overrides for ambiguous or
non-kebab leaves; require explicit confirmation against the reviewed digest;
and report per-item results with recovery guidance. All rendering SHALL be
text-only with labelled controls, live-region status, keyboard operation and
honest loading/empty/error/permission/blocked states.

#### Scenario: Operator onboards new siblings in the browser

- **WHEN** a signed-in operator discovers, selects, previews and confirms candidates
- **THEN** the browser shows per-item outcomes and the new projects appear in the fleet via their registry records

#### Scenario: Onboarding evidence is unavailable or blocked

- **WHEN** discovery cannot run or a candidate cannot be onboarded
- **THEN** the browser shows an honest unavailable or blocked state with the exact next step and invents no progress
