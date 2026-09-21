# provider-integration-evidence Specification

## Purpose

Prove existing external adapter boundaries with controlled runtime evidence,
while preserving truthful unavailable, partial and not-run states.

## ADDED Requirements

### Requirement: Controlled provider round trips

Forge SHALL support opt-in controlled round trips for the configured policy,
identity, analytics and deployment/release provider boundaries and SHALL record
provider, project, revision, timestamp and redacted receipt provenance.

#### Scenario: Provider round trip succeeds

- **WHEN** a configured sandbox provider returns a valid project-scoped result
- **THEN** Forge records the observation with provenance and exposes the same result through each available transport

#### Scenario: Provider is unavailable or not configured

- **WHEN** the provider binary, sandbox or credentials are unavailable
- **THEN** Forge reports unavailable or not-run evidence and never fabricates a healthy result

#### Scenario: Evidence is repeated

- **WHEN** the same provider check is repeated for the same project revision
- **THEN** the result remains attributable to its observation timestamp and does not overwrite unrelated provider evidence

### Requirement: Negative and partial provider outcomes

Forge SHALL test and preserve project mismatch, authorization failure, expiry,
revocation, timeout, malformed output, stale observation and partial-stage
outcomes.

#### Scenario: Authorization or mapping fails

- **WHEN** a token belongs to another project or a provider reports another project reference
- **THEN** Forge refuses the binding with a structured mismatch/authorization outcome

#### Scenario: Provider execution fails

- **WHEN** a provider times out, exits non-zero or emits malformed output
- **THEN** Forge records unavailable/failed evidence with redacted diagnostics and no success state

#### Scenario: One stage is partial

- **WHEN** one release or deployment provider succeeds and another fails
- **THEN** the aggregate remains partial and retry does not erase or repeat the completed stage blindly
