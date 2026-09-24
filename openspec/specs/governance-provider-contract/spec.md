# governance-provider-contract Specification

## Purpose
Provide a standalone-first, versioned governance-provider boundary so Forge
can operate locally while optionally consuming replaceable external governance
systems and their timestamped evidence.
## Requirements
### Requirement: Local provider independence

Forge SHALL provide a built-in local governance provider that is selected when
no external provider is configured and SHALL support local project inspection,
doctor, registry, and operation workflows without a sibling repository,
network service, account, shared database, or external adapter binary.

#### Scenario: Standalone local operation

- **WHEN** Forge is run with no provider configuration and all external tools
  unavailable
- **THEN** local project workflows complete using the built-in provider and do
  not report the missing external tools as local failures

#### Scenario: Optional provider configuration

- **WHEN** a project has no provider configuration block
- **THEN** its existing manifest and registry remain valid and Forge selects
  the local provider without mutation

### Requirement: Versioned provider contract

Forge SHALL define a versioned provider contract that normalizes provider
identity, protocol version, project identity, status, observation timestamp,
source revision, evidence references, and bounded redacted findings without
making provider-specific models Core dependencies.

#### Scenario: Compatible observation

- **WHEN** a selected provider returns a valid response for the requested
  project and protocol version
- **THEN** Forge stores and renders the normalized observation with its provider,
  source revision, timestamp, status, and evidence references

#### Scenario: Incompatible response

- **WHEN** a provider returns an unsupported protocol version, unknown status,
  malformed payload, or unbounded response
- **THEN** Forge reports an incompatible or unavailable provider result and
  does not record it as healthy or PASS

### Requirement: Explicit provider selection and replacement

Forge SHALL allow users to inspect, select, disable, and replace a governance
provider without rewriting the canonical project manifest, project identity,
registry records, or external provider state.

#### Scenario: Provider selection

- **WHEN** a user selects an installed compatible provider
- **THEN** Forge records the selection and uses it for subsequent governance
  observations while preserving local and historical observations

#### Scenario: Provider replacement

- **WHEN** a user replaces one provider with another
- **THEN** prior observations remain attributable to the original provider and
  current status is sourced only from the newly selected provider

### Requirement: Provider failure isolation

Forge SHALL isolate optional provider failures from local operations and SHALL
  surface disabled, unavailable, stale, timeout, identity-mismatch, and
  incompatible states explicitly.

#### Scenario: Provider unavailable

- **WHEN** the selected external adapter is missing, times out, or exits with a
  failure
- **THEN** Forge reports an unavailable observation with bounded remediation
  evidence and local commands remain usable

#### Scenario: Provider identity mismatch

- **WHEN** a provider response identifies a different project than the explicit
  Forge target
- **THEN** Forge rejects the observation without attaching it to either
  project's healthy state

#### Scenario: Provider-declared stale observation

- **WHEN** the provider response declares its evidence stale
- **THEN** Forge preserves the stale status and original timestamp and does not
  render it as current PASS evidence

### Requirement: Transport parity and optional adapter boundary

Forge SHALL expose normalized governance observations consistently through CLI,
MCP, API, and portal surfaces, and an external adapter SHALL be optional to the
base Forge installation.

#### Scenario: Local transport parity

- **WHEN** the local provider returns an observation
- **THEN** CLI, MCP, API, and portal surfaces render equivalent normalized
  status and provenance

#### Scenario: External adapter absence

- **WHEN** the optional external adapter and its sibling repository are absent
- **THEN** Forge still builds and the local provider path remains operational

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
