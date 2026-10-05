## ADDED Requirements

### Requirement: A request write the child closed is not an adapter failure

Forge SHALL write the request of every external adapter, translator and
provider to its standard input through one shared boundary, SHALL treat a
`BrokenPipe` on that write as a normal completion of the write step — it can
only mean the child closed its input — and SHALL continue to the child's real
exit status, stdout and stderr so that an adapter which answers without reading
its request is reported by what it actually answered. Forge SHALL keep a typed
refusal for every other request-write failure, and SHALL kill and reap the
child before returning that refusal.

#### Scenario: An adapter answers without reading its request

- **WHEN** a publish provider, translator or delivery adapter exits without
  reading the request Forge wrote to its standard input
- **THEN** Forge reads that exit status and output, reports the adapter's own
  status, and does not report a publish, translation or delivery failure
  because Forge lost the write race

#### Scenario: The write race repeats identically

- **WHEN** the same adapter that does not read its request is invoked
  repeatedly, including while other adapter tests run concurrently
- **THEN** every invocation returns the same status and the same evidence, and
  no invocation reports a broken pipe

#### Scenario: The request cannot be written for a real reason

- **WHEN** writing the request fails for any reason other than the child
  closing its input
- **THEN** Forge terminates and reaps the child and returns the typed
  unavailable, publish-invalid or translation-failed refusal naming the
  failure

#### Scenario: A write failure is never silently swallowed

- **WHEN** the request write fails for any reason
- **THEN** the outcome is decided by Forge's own code path and never by an
  adapter's exit status alone, so a request that was not delivered cannot be
  reported as an adapter answer
