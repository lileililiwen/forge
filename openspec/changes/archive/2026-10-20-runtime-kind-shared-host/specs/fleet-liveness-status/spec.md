## MODIFIED Requirements

### Requirement: Read-only fleet online verdicts
Forge SHALL provide `forge fleet online` which joins, per publishable roster
entry — one the fleet would have published: compose-ready and an own-process
runtime kind — the target container state, the served router rules, and one
bounded HTTPS probe into a typed verdict of `ONLINE`, `DOWN`, `NO-ROUTE`, or
`NOT-DEPLOYED`, without writing the journal, registry, or target and without
reading secret values. An entry the fleet never published SHALL be reported
with its classification and host model and SHALL NOT be probed, so a module
never reads as a service that failed to start.

#### Scenario: Online fleet renders all-online
- **WHEN** every routed host answers through its public URL with a non-fallback, non-gateway-error status and every container is present
- **THEN** the report verdicts every entry `ONLINE`, exits 0, and the JSON carries the `forge-fleet-liveness/0.1.0` contract

#### Scenario: Down host is typed, not silent
- **WHEN** a routed host times out, refuses connection, or answers 502/503/504
- **THEN** that entry verdicts `DOWN` with the bounded cause as detail and the command exits non-zero

#### Scenario: Missing route or container is explicit
- **WHEN** a roster project has no served router rule or no target container
- **THEN** it verdicts `NO-ROUTE` or `NOT-DEPLOYED` respectively, is never probed further than needed, and the command exits non-zero

#### Scenario: A module is never probed
- **WHEN** a roster entry declares a shared-host runtime kind and is compose-ready
- **THEN** it is reported as `SKIPPED` with its host model and the reason that the project which adopts it owns the lifecycle, and it is never probed

#### Scenario: Read-only guarantee
- **WHEN** `forge fleet online` runs against any target
- **THEN** no journal row is appended, the registry bytes are unchanged, no target file is written, and no secret value appears in any output
