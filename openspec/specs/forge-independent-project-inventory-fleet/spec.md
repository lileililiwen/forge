# forge-independent-project-inventory-fleet Specification

## Purpose
A portable `forge-project-inventory/0.1.0` source that lets `forge publish fleet` classify and publish a roster independently of the workspace-governance `projects.json`.
## Requirements
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
source-unavailable and SHALL not silently omit entries. Every reported entry
whose runtime kind was read SHALL name that kind and the host model the kind
declares, and a shared-host entry SHALL carry the reason that it is a module
hosted by the project that adopts it. An entry whose runtime kind could not be
read SHALL name no runtime kind and no host model, and SHALL carry the reason
it was refused.

#### Scenario: Missing Compose

- **WHEN** an inventory entry has no declared Compose file
- **THEN** the fleet result SHALL include that project as `compose_missing`
  without invoking a provider for it

#### Scenario: A shared-host entry is reported as a module

- **WHEN** an inventory entry declares a shared-host runtime kind
- **THEN** the fleet result SHALL name its host model and SHALL state that it
  is a module hosted by the project that adopts it

#### Scenario: A refused entry names no runtime kind

- **WHEN** an inventory entry is refused before its runtime kind can be read
- **THEN** the fleet result SHALL name no runtime kind and no host model for
  it, and SHALL state the reason it was refused

### Requirement: Container fleet publishing

Forge SHALL publish every valid Compose-ready inventory entry whose runtime
kind is an own-process kind as a Mac Docker Compose workload, including
non-web own-process runtime classes. Forge SHALL NOT publish a shared-host
runtime class as a workload of its own, whatever Compose file it declares,
because the host that adopts the module owns its lifecycle.

#### Scenario: Worker without HTTP port

- **WHEN** a worker has a valid Compose contract but no public HTTP port
- **THEN** Forge SHALL run the worker container and SHALL report no public URL

#### Scenario: A module is held back

- **WHEN** a library entry has a valid Compose contract
- **THEN** Forge SHALL report it, SHALL name its host model, and SHALL NOT
  run it as a workload of its own

### Requirement: Public port routing

Forge SHALL register only explicitly public HTTP ports in the Mac runtime
registry and SHALL route them through Caddy behind the existing wildcard
Cloudflare Tunnel. Only an own-process runtime class may declare a public
HTTP port; a shared-host runtime class SHALL be refused, and the refusal
SHALL name the host model.

#### Scenario: Web service with public port

- **WHEN** a project declares a public HTTP service and reaches healthy run
  state
- **THEN** Forge SHALL make `<project>.tooosall.uk` route to that service

#### Scenario: Private database port

- **WHEN** a Compose project contains PostgreSQL, Redis, or another private
  service port
- **THEN** that port SHALL not receive a Cloudflare/Caddy public route

#### Scenario: A module may not expose HTTP

- **WHEN** an entry declares a shared-host runtime class and declares
  `public_http`
- **THEN** Forge SHALL refuse the entry, and the refusal SHALL name the host
  model

### Requirement: Standalone ownership

Forge SHALL remain operational when workspace-governance is absent, relocated,
or replaced by another inventory provider.

#### Scenario: Workspace-governance is relocated

- **WHEN** its adapter is configured with a new explicit executable and root
- **THEN** Forge SHALL use that adapter without code changes or a fixed path
  assumption

