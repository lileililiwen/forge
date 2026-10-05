## ADDED Requirements

### Requirement: Adapter request-write race is not a provider failure

Forge SHALL treat a request write that the adapter closed before consuming it —
signalled by `BrokenPipe` — as a normal completion of the write step and SHALL
continue to the adapter's real exit status, stdout and stderr, so that a
provider which answers without reading its request is reported by what it
actually answered. Forge SHALL keep its typed unavailable refusal for every
other request-write failure, and SHALL kill and reap the adapter child before
returning that refusal.

#### Scenario: Adapter answers without reading its request

- **WHEN** a selected external adapter exits successfully without reading the
  request Forge wrote to its standard input
- **THEN** Forge reads that exit status and response, reports the provider's
  own status, and does not report `unavailable` because Forge lost the write
  race

#### Scenario: Request cannot be written for a real reason

- **WHEN** writing the adapter request fails for any reason other than the
  adapter closing its input
- **THEN** Forge terminates and reaps the adapter child and reports an
  unavailable provider result with a bounded detail naming the failure

#### Scenario: Repeated checks of a non-reading adapter agree

- **WHEN** the same adapter that does not read its request is selected and
  checked repeatedly, including while other checks run concurrently
- **THEN** every check returns the same status and the same evidence, and no
  check reports `unavailable`
