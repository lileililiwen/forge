# Capability: independent project inventory and container fleet

## ADDED Requirements

### Requirement: Portable inventory source

Forge SHALL consume a versioned normalized project inventory from an explicitly
selected local or external provider and SHALL NOT require workspace-governance
or a fixed workspace path.

#### Scenario: Local inventory works alone

- **WHEN** Forge is given a valid local inventory file
- **THEN** it SHALL validate and publish it without contacting
  workspace-governance

#### Scenario: Optional adapter unavailable

- **WHEN** the selected external inventory adapter is disabled or unavailable
- **THEN** Forge SHALL return a typed source failure and SHALL not silently
  scan another sibling checkout

### Requirement: Complete inventory reporting

Forge SHALL report every inventory entry as ready, missing Compose, invalid, or
source-unavailable and SHALL not silently omit entries.

#### Scenario: Missing Compose

- **WHEN** an inventory entry has no declared Compose file
- **THEN** the fleet result SHALL include that project as `compose_missing`
  without invoking a provider for it

### Requirement: Container fleet publishing

Forge SHALL publish every valid Compose-ready inventory entry as a Mac Docker
Compose workload, including non-web runtime classes.

#### Scenario: Worker without HTTP port

- **WHEN** a worker has a valid Compose contract but no public HTTP port
- **THEN** Forge SHALL run the worker container and SHALL report no public URL

### Requirement: Public port routing

Forge SHALL register only explicitly public HTTP ports in the Mac runtime
registry and SHALL route them through Caddy behind the existing wildcard
Cloudflare Tunnel.

#### Scenario: Web service with public port

- **WHEN** a project declares a public HTTP service and reaches healthy run
  state
- **THEN** Forge SHALL make `<project>.tooosall.uk` route to that service

#### Scenario: Private database port

- **WHEN** a Compose project contains PostgreSQL, Redis, or another private
  service port
- **THEN** that port SHALL not receive a Cloudflare/Caddy public route

### Requirement: Standalone ownership

Forge SHALL remain operational when workspace-governance is absent, relocated,
or replaced by another inventory provider.

#### Scenario: Workspace-governance is relocated

- **WHEN** its adapter is configured with a new explicit executable and root
- **THEN** Forge SHALL use that adapter without code changes or a fixed path
  assumption

## Traceability

| Requirement | Design boundary | Verification |
|---|---|---|
| Portable inventory source | `src/publish/inventory.rs`, external adapter invocation | local/external adapter contract tests |
| Complete inventory reporting | fleet classifier and result model | 77-entry fixture with missing Compose |
| Container fleet publishing | Forge fleet + Jenkins Mac provider | serial provider fixture and Mac Compose evidence |
| Public port routing | runtime registry and Caddy renderer | public/private route tests and Cloudflare URL check |
| Standalone ownership | provider configuration, no implicit path search | relocated/missing adapter tests |
