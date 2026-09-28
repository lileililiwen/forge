# fleet-liveness-status Specification

## Purpose
TBD - created by archiving change fleet-liveness-status. Update Purpose after archive.
## Requirements
### Requirement: Read-only fleet online verdicts
Forge SHALL provide `forge fleet online` which joins, per `compose_ready` roster entry, the target container state, the served router rules, and one bounded HTTPS probe into a typed verdict of `ONLINE`, `DOWN`, `NO-ROUTE`, or `NOT-DEPLOYED`, without writing the journal, registry, or target and without reading secret values.

#### Scenario: Online fleet renders all-online
- **WHEN** every routed host answers through its public URL with a non-fallback, non-gateway-error status and every container is present
- **THEN** the report verdicts every entry `ONLINE`, exits 0, and the JSON carries the `forge-fleet-liveness/0.1.0` contract

#### Scenario: Down host is typed, not silent
- **WHEN** a routed host times out, refuses connection, or answers 502/503/504
- **THEN** that entry verdicts `DOWN` with the bounded cause as detail and the command exits non-zero

#### Scenario: Missing route or container is explicit
- **WHEN** a roster project has no served router rule or no target container
- **THEN** it verdicts `NO-ROUTE` or `NOT-DEPLOYED` respectively, is never probed further than needed, and the command exits non-zero

#### Scenario: Read-only guarantee
- **WHEN** `forge fleet online` runs against any target
- **THEN** no journal row is appended, the registry bytes are unchanged, no target file is written, and no secret value appears in any output

### Requirement: Served router rules are the route ground truth
Route presence SHALL be determined by parsing the served `platform/Caddyfile` host rules; the nav host and the `:80` fallback SHALL be excluded from probing.

#### Scenario: Stale router is visible
- **WHEN** the served Caddyfile lacks a rule the local registry predicts
- **THEN** the project verdicts `NO-ROUTE` with detail naming the absent host

### Requirement: Application answers count as online
Any non-fallback response status other than 502/503/504 (including application-level 4xx such as an API-only service's `/`) SHALL verdict `ONLINE`; only the router's unknown-hostname fallback body and gateway errors SHALL verdict `DOWN`.

#### Scenario: API-only service on /
- **WHEN** a healthy app answers its public URL with its own 404 body
- **THEN** the entry verdicts `ONLINE`

