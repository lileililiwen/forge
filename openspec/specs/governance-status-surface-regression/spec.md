# governance-status-surface-regression Specification

## Purpose

Keep governance provider observations and Forge Gate quality evidence attributed to their respective owners when Forge projects them through existing read surfaces.
## Requirements
### Requirement: GSR-001 Governance and Gate status projections remain independent

Forge SHALL project selected governance provider observations through governance status and checker surfaces without treating the portal settings Gate row as a governance observation. The portal settings row SHALL derive its status and freshness only from Gate evidence; absent Gate evidence SHALL remain `unknown` with `freshness=absent`.

#### Scenario: Missing governance adapter does not change Gate status

- **WHEN** the selected governance adapter cannot be started and the project has no Gate evidence
- **THEN** governance status reports `unavailable`, the checker reports a warning attributed to the selected provider, and portal settings reports `unknown` with absent Gate freshness and no governance-provider attribute

#### Scenario: Provider failure does not change Gate status

- **WHEN** the selected governance adapter returns a compatible `fail` observation and the project has no Gate evidence
- **THEN** governance status reports `fail`, the checker reports an error attributed to the selected provider, and portal settings continues to report `unknown` with absent Gate freshness
