# runtime-hardening-and-test-isolation Specification

## Purpose
Cross-cutting runtime and test hardening: bounded subprocess execution with kill-and-reap on timeout, and registry-backed transport tests isolated on temporary registries so host state (including a read-only HOME) cannot leak into results.
## Requirements
### Requirement: Isolated registry-backed test execution

Forge SHALL ensure tests that exercise registry-backed transports use an
explicit disposable registry and SHALL NOT depend on the host's default
registry path.

#### Scenario: Isolated round trip succeeds

- **WHEN** an MCP session handles a read request in a writable temporary test workspace
- **THEN** it returns the expected response without reading or writing the host registry

#### Scenario: Read-only host state is harmless

- **WHEN** the default home or data directory is read-only
- **THEN** the isolated test still runs against its temporary registry and does not fail with a host `readonly database` error

#### Scenario: Sequential sessions remain independent

- **WHEN** two tests or sessions run sequentially with different temporary registries
- **THEN** neither observes projects or operations from the other

### Requirement: Bounded release subprocess cleanup

Forge SHALL kill and reap a release adapter child before returning a timeout
failure and SHALL never record the timed-out stage as successful.

#### Scenario: Adapter completes

- **WHEN** a release adapter exits before its deadline with valid output
- **THEN** Forge returns its output and preserves the existing successful stage contract

#### Scenario: Adapter times out

- **WHEN** a release adapter exceeds its deadline
- **THEN** Forge terminates and reaps the direct child, returns a typed unavailable/timeout outcome, and records no successful stage

#### Scenario: Adapter fails or emits invalid output

- **WHEN** an adapter exits non-zero or returns an invalid receipt
- **THEN** Forge reports the failure without leaking raw credential-shaped diagnostics or claiming completion

### Requirement: Studio test port ranges are chosen at run time outside the host ephemeral window

Forge SHALL NOT hardcode a TCP port range in any test that drives the Studio
preview allocator or that exercises a process-global port-range setting. Those
tests SHALL share one selection of a width-wide window, made at run time from
candidates lying wholly outside the host's ephemeral port window, every port in
the chosen window SHALL have been bindable when it was chosen, and each
assertion about the allocated port SHALL be computed from the base that was
actually configured rather than from a literal. A test that needs its range
occupied SHALL hold the listeners that verified the window was free, so the
choice and the occupancy cannot race. Each Studio target SHALL begin its search
at its own candidate index, so targets running at the same time do not prefer the
same window.

#### Scenario: An unrelated outbound connection takes a port

- **WHEN** any process on the host draws an ephemeral source port from the
  range a Studio test used to hardcode
- **THEN** the Studio tests are unaffected, because their window is chosen
  outside the host's ephemeral range and no unrelated connection can take one
  of its ports

#### Scenario: A candidate range is partly taken

- **WHEN** a candidate range cannot be bound in full
- **THEN** the selection moves to the next candidate outside the ephemeral
  range and the assertion under test runs unchanged

#### Scenario: Every candidate range is taken

- **WHEN** no candidate range outside the ephemeral window can be bound in full
- **THEN** the test fails, naming every candidate it tried, rather than
  proceeding with fewer ports than its range requires

#### Scenario: The allocated port leaves the configured window

- **WHEN** a preview session reports a port outside the window the test
  configured
- **THEN** the test fails and names the configured base, so a drifting
  allocator cannot pass a range assertion

#### Scenario: Two Studio targets run at the same time

- **WHEN** two Studio test binaries that drive the allocator run concurrently
- **THEN** each prefers a different candidate window, so neither target's runner
  is handed a port the other is already holding

#### Scenario: A chosen window is claimed before the runner binds it

- **WHEN** another process takes a port between a test verifying its window is
  free and its runner binding that port
- **THEN** the failure is Forge's own allocator window, not the test's port
  choice, and it is recorded as a product defect rather than a test range
  defect
