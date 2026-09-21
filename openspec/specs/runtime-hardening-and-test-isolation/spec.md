# runtime-hardening-and-test-isolation Specification

## Purpose
TBD - created by archiving change runtime-hardening-and-test-isolation. Update Purpose after archive.
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

