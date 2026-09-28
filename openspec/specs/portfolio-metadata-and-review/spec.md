# portfolio-metadata-and-review Specification

## Purpose

Give a portfolio the horizontal information repository metadata
cannot express: lifecycle, confidence, tags, goals, project
relationships, blockers, next actions and review history, kept
separate from repository- and provider-owned facts. Source-owned
observations (governance, gate, release, runtime) arrive only as
append-only, source-labelled snapshots whose `stale` and
`unavailable` states are preserved verbatim. The read model is
served by the existing `forge portfolio` CLI, the project-scoped
JSON API routes and the existing authenticated portal, and no
surface edits repository files or provider records.

## Requirements
### Requirement: Store horizontal portfolio metadata

Forge SHALL persist user-owned lifecycle, confidence, tags, goals, relations,
blockers, next actions, and review history separately from repository and
provider-owned facts.

#### Scenario: Tag and review project

- **WHEN** an operator assigns a valid tag and review to an imported project
- **THEN** Forge persists both with the project identity and review timestamp

#### Scenario: Unknown project

- **WHEN** an operator writes metadata for an unknown project id
- **THEN** Forge returns typed not-found and changes no portfolio state

### Requirement: Enforce relation integrity

Forge SHALL validate relation types, project identities, duplicate relations,
and self-relations before persistence.

#### Scenario: Valid dependency relation

- **WHEN** two known projects are linked with `depends-on`
- **THEN** one idempotent relation is stored and shown in project views

#### Scenario: Self relation

- **WHEN** a project is linked to itself
- **THEN** Forge refuses the relation with a typed validation error

### Requirement: Import source-owned evidence as snapshots

Forge SHALL import governance, gate, release, and runtime observations as
source-labelled snapshots containing source system, source revision, observed
time, status, and redacted evidence.

#### Scenario: Fresh evidence

- **WHEN** a valid provider observation is imported
- **THEN** Forge stores it as an observed snapshot and displays its source and
  revision without claiming Forge performed the check

#### Scenario: Stale or unavailable provider

- **WHEN** an observation exceeds its freshness bound or its provider is
  unavailable
- **THEN** the read model displays `stale` or `unavailable`, never `healthy`

### Requirement: Project portfolio data through the portal

Forge SHALL expose portfolio filters and project detail fields through the
existing authenticated portal without mutating repository or provider data.

#### Scenario: Filter by tag and lifecycle

- **WHEN** an authorized operator filters the portfolio by tag and lifecycle
- **THEN** matching projects and their evidence states are rendered

#### Scenario: Unauthorized mutation

- **WHEN** an unauthorized request attempts to change portfolio metadata
- **THEN** the existing API authorization boundary refuses it and persists no
  change
