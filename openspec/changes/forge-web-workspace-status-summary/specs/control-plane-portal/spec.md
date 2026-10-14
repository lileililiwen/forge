# control-plane-portal Specification (delta)

## ADDED Requirements

### Requirement: Topbar workspace status summary

Forge SHALL provide, in the portal topbar of the "All projects" view, a read-only
summary that mirrors the CLI workspace sibling overview: the count of sibling
directories **registered** in the registry, the count **onboardable** (not yet
registered), and a **last-sync** freshness label. The summary SHALL aggregate
from the existing session-gated `GET /v1/admin/workspace/candidates` route and
SHALL introduce no new API route, no write control, and no Core capability.

#### Scenario: Signed-in operator sees workspace counters

- **WHEN** an authenticated admin with a configured `FORGE_ADMIN_PROJECTS_ROOT` opens "All projects"
- **THEN** the topbar shows the registered count (siblings whose candidate `state` is `registered`) and the onboardable count (candidates whose `selectable` is true) aggregated from one candidate request, and a last-sync label derived from the client observation time

#### Scenario: Workspace root is unconfigured

- **WHEN** `FORGE_ADMIN_PROJECTS_ROOT` is unset, blank or not a directory
- **THEN** the candidate route refuses with typed `409 admin-prerequisite` and the topbar shows an honest "Workspace not configured" chip instead of any count — never a fabricated number

#### Scenario: Candidate read fails for another reason

- **WHEN** the candidate request fails with a non-409 transport or auth error
- **THEN** the counters are hidden and a status note reports the workspace summary is unavailable, without claiming a count

#### Scenario: Last-sync becomes stale

- **WHEN** the summary has not been re-read within the bounded idle threshold
- **THEN** the last-sync label switches to a `stale` state and retains its last observation time

#### Scenario: No write control in the summary

- **WHEN** the operator views the topbar summary
- **THEN** no onboard, link or mutation control is present — the summary is a read-only mirror of the candidate route
