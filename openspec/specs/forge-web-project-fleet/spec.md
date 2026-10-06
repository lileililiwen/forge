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

