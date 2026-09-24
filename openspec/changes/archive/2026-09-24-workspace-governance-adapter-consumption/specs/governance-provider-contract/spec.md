# governance-provider-contract Specification

## ADDED Requirements

### Requirement: Known-provider preset resolution

Forge SHALL resolve a packaged adapter location for a known governance
provider id from explicit operator input only — a `--workspace-root`
argument or the `FORGE_WORKSPACE_ROOT` environment variable — SHALL verify
the candidate is an existing executable regular file before selecting it,
and SHALL never search parent directories or the network for a provider.

#### Scenario: Preset selection

- **WHEN** an operator runs `forge governance use workspace-governance .`
  with a workspace root whose scripts directory holds the adapter
- **THEN** selection succeeds, stores the resolved adapter path, and the
  next status check runs through the existing v0.1.0 boundary

#### Scenario: Preset unresolved

- **WHEN** the candidate path does not exist or is not executable
- **THEN** the command refuses, names the exact candidate path, and the
  previously selected provider remains in force

#### Scenario: Explicit adapter wins

- **WHEN** both `--adapter` and a resolvable preset are supplied
- **THEN** the explicit path is used and preset resolution does not override
  the operator's choice

### Requirement: Provider input isolation

Forge SHALL keep local workflows fully functional when a known provider is
configured but its workspace root is absent, and SHALL classify the
resulting observation as unavailable or incompatible without reading or
mutating any sibling repository outside the adapter boundary.

#### Scenario: Sibling checkout missing

- **WHEN** provider selection points at an adapter whose file was later
  removed
- **THEN** `forge governance status` reports unavailable with bounded
  detail and every local command continues unchanged

#### Scenario: Audit-mapped statuses

- **WHEN** the adapter returns observations mapped from portfolio audit
  results (`pass`, `fail`, `blocked`, `unknown`)
- **THEN** Forge renders each under the existing normalized vocabulary with
  redacted evidence and never converts an absent adapter into a PASS
