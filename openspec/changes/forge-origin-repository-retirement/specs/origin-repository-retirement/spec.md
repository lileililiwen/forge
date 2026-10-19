# Origin repository retirement Specification

## Purpose

Retire writable remote origins only after Forge source imports, history and
consumers are verified, while preserving local user work and unrelated
repositories.

## Requirements

## ADDED Requirements

### Requirement: Reconcile migrated source origins against Forge ownership

The retirement workflow SHALL use an explicit allowlist mapping each migrated
origin to its Forge-owned source path. It SHALL verify available import
provenance, preserved source-only history, and maintained Forge consumers
before any remote write. It SHALL re-query live GitHub state immediately
before acting and classify repositories as archived, active, confirmed absent
or unavailable. It SHALL NOT treat authentication, network, rate-limit or
permission errors as absence.

#### Scenario: A required source import or consumer check fails

- **WHEN** a source verifier, source-history ancestry check or maintained
  consumer check fails
- **THEN** the workflow reports the exact failure and performs no remote
  archival operation for the affected origin

#### Scenario: A repository lookup is unavailable

- **WHEN** the GitHub API cannot establish the repository state
- **THEN** the workflow reports it as unresolved and does not infer absence or
  perform an archive write

### Requirement: Archive only active allowlisted migrated origins

The retirement workflow SHALL archive only an existing, active repository in
the migrated-origin allowlist. It SHALL leave already archived repositories
unchanged, record confirmed missing origins as absent, and verify every
archive write by reading back `archived=true`. It SHALL NOT delete, transfer,
rename, create, or rewrite a repository, nor act on unrelated platform,
runtime or consumer repositories.

#### Scenario: An allowlisted repository is active

- **WHEN** source and consumer preconditions pass and the exact repository is
  confirmed active immediately before the operation
- **THEN** only that repository is archived, and completion is reported only
  after the GitHub read-back confirms `archived=true`

#### Scenario: An allowlisted repository is already archived or absent

- **WHEN** a fresh authoritative lookup finds `archived=true` or confirms the
  repository is absent
- **THEN** no write occurs and the exact state is recorded as
  `already_archived` or `absent`

#### Scenario: The archive read-back does not confirm the write

- **WHEN** an archive request fails or the subsequent state is not
  `archived=true`
- **THEN** that origin remains unresolved, no success is claimed, and no
  unrelated repository is modified

### Requirement: Preserve local checkouts and unrelated repositories

The workflow SHALL preserve each pre-existing local sibling checkout and its
worktree contents, including untracked files. It SHALL capture local status
before and after remote archival and SHALL NOT clean, reset, delete, or modify
those checkouts. It SHALL exclude all non-allowlisted repositories.

#### Scenario: A source checkout contains untracked user files

- **WHEN** the checkout is inventoried before remote archival
- **THEN** the untracked files remain untouched and the same worktree status is
  observed after the remote operation

#### Scenario: An unrelated repository is named during inventory

- **WHEN** a repository is not in the explicit migrated-origin allowlist
- **THEN** it is recorded as out of scope and receives no archive request

### Requirement: Report a per-origin retirement result

HANDOFF SHALL record the observation time, exact origin, outcome, provenance
and consumer evidence, local-checkout preservation evidence, and any unresolved
state. The workflow SHALL report partial completion per origin and SHALL NOT
represent unavailable or failed origins as retired.

#### Scenario: Retirement completes with mixed outcomes

- **WHEN** some origins are newly archived, already archived, or confirmed
  absent
- **THEN** each origin is reported with its own verified state and no aggregate
  success hides an unresolved origin
