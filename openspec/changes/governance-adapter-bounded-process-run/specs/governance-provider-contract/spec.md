## ADDED Requirements

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
