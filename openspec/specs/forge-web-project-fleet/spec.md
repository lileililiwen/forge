# forge-web-project-fleet Specification

## Purpose
TBD - created by archiving change forge-web-project-fleet. Update Purpose after archive.
## Requirements
### Requirement: Complete authenticated fleet aggregation

Forge SHALL return the Forge-self record, every local registered project, and every valid entry in each explicitly selected external inventory source in one stable authenticated fleet response.

#### Scenario: Fleet includes all sources and Forge

- **WHEN** an authenticated operator opens the fleet with local projects and an external inventory configured
- **THEN** the response contains Forge, every local project and every valid external entry with source and management capability labels

#### Scenario: Forge is not registered

- **WHEN** the Forge project is absent from the local registry
- **THEN** the response still contains exactly one Forge-self entry marked `is_self=true`

#### Scenario: Anonymous fleet request

- **WHEN** an anonymous or expired session requests the fleet
- **THEN** Forge returns 401 and reveals no source or project data

### Requirement: Explicit sources and truthful partial states

Forge SHALL read only explicitly configured sources and SHALL report healthy-empty, stale, malformed, unavailable and unconfigured states without silently omitting declared entries.

#### Scenario: External source unavailable

- **WHEN** one configured source cannot be read while another source is healthy
- **THEN** the healthy entries remain visible and the failed source is represented as unavailable with a safe reason

#### Scenario: Stale source

- **WHEN** the configured source exceeds its freshness bound
- **THEN** its entries remain visible with stale status and observation time

#### Scenario: No implicit workspace scan

- **WHEN** no external source is configured
- **THEN** Forge returns local projects and itself without scanning sibling directories or guessing a registry path

### Requirement: Identity conflicts and management boundaries

Forge SHALL preserve conflicting source records with provenance and SHALL expose mutating capabilities only for Forge-managed projects.

#### Scenario: Conflicting source identity

- **WHEN** two sources declare the same canonical identity with conflicting metadata
- **THEN** both records are visible with a conflict marker and ambiguous mutation links are disabled

#### Scenario: Observed-only project

- **WHEN** a project exists only in a read-only external source
- **THEN** the web UI permits inspection of source evidence but offers no Forge mutation action

### Requirement: Local publish history is projected into the fleet

Forge SHALL project the most recent `publish` operation per `project_id`
from the local registry `operations` table into the authenticated fleet as
a bounded, read-only source, so projects the operator has published but not
locally registered appear in the web project list.

#### Scenario: Published-only project appears

- **WHEN** the local journal has publish operations for a project id that is
  not in the local `projects` table
- **THEN** the fleet contains one row for that id with `source` `published`,
  `management` `observed`, no mutating capability, and the most recent
  publish state and bounded publish evidence

#### Scenario: Registered and published project stays managed

- **WHEN** a project id is both locally registered and present in the publish
  journal
- **THEN** the fleet contains a single row for that id that remains
  `management` `managed`, keeps its `inspect` capability, is not marked as a
  conflict, and carries the publish projection

#### Scenario: Most recent publish wins

- **WHEN** the journal holds several publish operations for one project id
- **THEN** the row reflects only the newest operation, ordered by `op_id`

#### Scenario: Empty publish journal

- **WHEN** the registry has no publish operations
- **THEN** the `published` source is reported available with a zero count and
  contributes no rows, and the other sources are unchanged

#### Scenario: Publish history disabled

- **WHEN** `FORGE_PUBLISH_HISTORY` is set to `0`, `false` or `off`
- **THEN** the `published` source is reported unconfigured with a safe reason
  and contributes no rows, and the other sources are unchanged

#### Scenario: Publish evidence is path-safe

- **WHEN** a projected publish `detail` contains an absolute local filesystem
  path
- **THEN** the serialized response replaces that path with a fixed marker and
  no absolute path, credential or adapter binary reaches the browser

