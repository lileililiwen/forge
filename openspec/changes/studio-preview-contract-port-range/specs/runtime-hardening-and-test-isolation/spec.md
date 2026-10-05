## ADDED Requirements

### Requirement: Port-range tests choose a run-time window

Forge SHALL NOT hardcode a fixed TCP port range in a test that exercises a
process-global port-range setting. The test SHALL read the host's ephemeral
port window, SHALL consider only candidate ranges lying wholly outside it, and
SHALL use one candidate for which every port in the width-wide window can be
bound, so that an unrelated outbound connection on the host can never take a
port the test depends on. A test that needs its range occupied SHALL hold the
listeners that verified the range was free, so the choice and the occupancy
cannot race.

#### Scenario: An outbound connection takes a port in a hardcoded range

- **WHEN** any process on the host draws an ephemeral source port from the
  range a test hardcoded
- **THEN** the test is unaffected: it operates on a window chosen outside the
  host's ephemeral range, so no unrelated connection can take one of its ports

#### Scenario: A candidate range is partly taken

- **WHEN** a candidate range cannot be bound in full
- **THEN** the test moves to the next candidate outside the ephemeral range and
  the assertion under test runs unchanged

#### Scenario: Every candidate range is taken

- **WHEN** no candidate range outside the ephemeral window can be bound in full
- **THEN** the test fails, naming every candidate it tried, rather than
  proceeding with fewer ports than its range requires
