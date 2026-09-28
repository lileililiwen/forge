# Interest snapshots

## ADDED Requirements

### Requirement: Forge accepts only privacy-safe aggregate snapshots
Forge SHALL accept versioned snapshots containing allowlisted non-negative counts
and provenance, and SHALL reject identity, raw event, payment, and credential
fields.

#### Scenario: Raw identity is submitted
- GIVEN an import contains an email field
- WHEN validation runs
- THEN the record is rejected and the email is not persisted or echoed

### Requirement: Snapshot evidence is immutable and idempotent
Forge SHALL preserve accepted snapshots and treat an exact duplicate as an
idempotent result.

#### Scenario: Adapter retries an import
- GIVEN a snapshot with the same project, window, source, and revision exists
- WHEN it is imported again
- THEN no second record is created

### Requirement: Comparisons preserve window semantics
Forge SHALL label source and freshness and SHALL NOT sum overlapping windows or
present stale evidence as current.

#### Scenario: Overlapping windows
- GIVEN two same-source windows overlap
- WHEN comparison data is requested
- THEN the system rejects the overlapping import or marks it as a replacement,
  and never double-counts it
