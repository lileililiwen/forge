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

### Requirement: Adapter request-write race is not a provider failure

Forge SHALL write the request of every external governance adapter, publish
provider, translator and delivery adapter to its standard input through one
shared boundary, SHALL treat a request write that the child closed before
consuming it — signalled by `BrokenPipe` — as a normal completion of the write
step, and SHALL continue to the child's real exit status, stdout and stderr, so
that a provider which answers without reading its request is reported by what it
actually answered. Forge SHALL keep a typed refusal for every other
request-write failure and SHALL terminate and reap the child before returning
that refusal. No call site SHALL discard a request-write error.

#### Scenario: Adapter answers without reading its request

- **WHEN** a selected external adapter, publish provider, translator or delivery
  adapter exits successfully without reading the request Forge wrote to its
  standard input
- **THEN** Forge reads that exit status and response, reports the adapter's own
  status and evidence, and does not report `unavailable`, a publish refusal, a
  translation failure or a delivery failure because Forge lost the write race

#### Scenario: Repeated checks of a non-reading adapter agree

- **WHEN** the same adapter that does not read its request is invoked
  repeatedly, including while other adapter tests run concurrently
- **THEN** every invocation returns the same status and the same evidence, and no
  invocation reports a broken pipe

#### Scenario: Request cannot be written for a real reason

- **WHEN** writing the request fails for any reason other than the child closing
  its input
- **THEN** Forge terminates and reaps the child and returns its typed
  unavailable, publish-invalid, translation-failed or delivery-unavailable
  refusal with a bounded detail naming the failure

#### Scenario: A write failure is never silently swallowed

- **WHEN** the request write fails for any reason
- **THEN** the outcome is decided by Forge's own code path and never by an
  adapter's exit status alone, so a request that was not delivered cannot be
  reported as an adapter answer

### Requirement: Bounded adapter subprocess run

Forge SHALL execute a selected external governance adapter with both of its
output pipes drained concurrently with the wait for its exit, SHALL bound the
wait by the selected provider's configured `timeout_ms`, and SHALL keep the
read buffer bounded by `MAX_ADAPTER_OUTPUT_BYTES`. Forge SHALL wait by
blocking on a bounded receive rather than by a fixed-interval poll, and SHALL
terminate and reap the adapter child before returning any timeout, wait-failure
or read-failure result.

#### Scenario: Adapter answers with more output than one pipe buffer holds

- **WHEN** a selected external adapter exits successfully after writing more
  bytes to stdout than the OS pipe buffer can hold, and no more than
  `MAX_ADAPTER_OUTPUT_BYTES`
- **THEN** Forge reads that output without the adapter blocking, reports the
  provider's own status and evidence from the complete response, and does not
  report a timeout

#### Scenario: Adapter exceeds the output cap

- **WHEN** a selected external adapter writes more than
  `MAX_ADAPTER_OUTPUT_BYTES` to stdout
- **THEN** Forge refuses with a typed governance-invalid result naming the cap,
  and does not buffer the excess

#### Scenario: Adapter exceeds its deadline

- **WHEN** a selected external adapter does not exit within the selected
  provider's `timeout_ms`
- **THEN** Forge terminates and reaps the adapter child before returning, and
  reports an unavailable observation whose detail names that timeout

#### Scenario: Adapter closes its pipes but keeps running

- **WHEN** a selected external adapter closes stdout and stderr and keeps
  running past its deadline
- **THEN** Forge still enforces the same deadline, terminates and reaps the
  child, and does not treat the closed pipes as the adapter having exited

#### Scenario: A fast adapter is not delayed

- **WHEN** a selected external adapter exits well inside its deadline
- **THEN** Forge returns its answer without waiting on a fixed-interval poll

### Requirement: Bounded source revision lookup

Forge SHALL resolve the recorded source revision through a subprocess bounded
by the selected provider's configured `timeout_ms`, SHALL bound the bytes it
reads from that subprocess, and SHALL record no revision rather than blocking
indefinitely when the lookup fails, times out or answers nothing usable.

#### Scenario: Revision lookup answers

- **WHEN** the project root is a Git repository whose `rev-parse HEAD` answers
  inside the bound
- **THEN** Forge records that object name as the observation's
  `source_revision`, as before

#### Scenario: Revision lookup never answers

- **WHEN** the revision lookup does not return within the bound
- **THEN** the governance check completes within that bound, records no
  `source_revision`, and does not block the caller

#### Scenario: Revision lookup fails or answers nothing usable

- **WHEN** the revision lookup cannot be started, exits non-zero, or answers no
  object name
- **THEN** Forge records no `source_revision` and the rest of the observation is
  unchanged
