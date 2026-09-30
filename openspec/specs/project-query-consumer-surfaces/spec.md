# project-query-consumer-surfaces Specification

## Purpose
One project-query service behind every transport, with stable machine output and equivalent authorization and typed failures across CLI, API and MCP.
## Requirements
### Requirement: One query service behind every transport

Forge SHALL expose the project catalog query contract through the CLI, MCP and
HTTP transports by delegating to a single Core query service, and SHALL NOT
implement filtering, ordering or pagination a second time in any transport.

#### Scenario: Transports agree

- **WHEN** the same catalog query is issued through the CLI, MCP and HTTP API
- **THEN** each returns the same records in the same order, and the only
  difference is the transport envelope

#### Scenario: A transport adds no rule

- **WHEN** a transport serializes a query result
- **THEN** it passes the Core result through unchanged and introduces no filter,
  ordering or default of its own

### Requirement: Stable machine output for pipelines

Forge SHALL provide a stable, versioned JSON document and a one-record-per-line
NDJSON stream for shell pipelines, and SHALL NOT use the human table as the
machine contract.

#### Scenario: NDJSON is stable

- **WHEN** a caller requests NDJSON
- **THEN** each record is one line in the deterministic Core order and repeated
  reads are byte-identical

#### Scenario: The table is not the contract

- **WHEN** the human table layout changes
- **THEN** the JSON and NDJSON contracts are unchanged

### Requirement: Authorization and typed failures are equivalent across transports

Forge SHALL enforce the same authorization and project-scope checks on every
transport, and SHALL return typed, equivalent errors for an invalid filter, an
unauthorized caller and a project mismatch.

#### Scenario: An unauthorized caller sees no fleet

- **WHEN** a catalog read is requested without a live authorized session
- **THEN** the API answers `401` and the CLI prints nothing to stdout, and the
  fleet composition is not disclosed

#### Scenario: An invalid filter is typed

- **WHEN** a caller supplies an unknown filter or value
- **THEN** the transport returns the same typed invalid error, and the CLI
  prints nothing to stdout

