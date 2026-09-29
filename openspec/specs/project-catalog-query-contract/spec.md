# project-catalog-query-contract Specification

## Purpose

Give one stable, read-only answer to "what projects exist and what does
each source actually know about them". Forge already observes a local
registry, an external workspace registry, a portable inventory and Git
working trees, but each surface answered on its own terms, so locating a
project or combining two observations meant running several
project-specific commands. The catalog is a **projection** over those
readers: it adds no table, no migration and no import, it never merges
two sources into one value, and it never presents an empty, stale or
unreadable source as a healthy one. Every value carries the source,
revision, observation and freshness it came from, and JSON/NDJSON are
the machine contract so the result can be piped into other commands
unchanged.

## Requirements
### Requirement: Normalized project catalog records with provenance

Forge SHALL expose a read-only project catalog, contract
`forge-project-catalog/0.1.0`, whose records carry the project identity,
classification, repository and stack facts, and the `source`,
`source_revision`, `observed_at` and `freshness` each value came from. A record
SHALL NOT contain a free-form metadata map, and an unknown field SHALL be
refused.

#### Scenario: A record names its source

- **WHEN** a project is read from a local registry, Git, workspace registry,
  explicit inventory or GitHub source
- **THEN** its record carries that source, the source revision and an
  observation timestamp, and `freshness` is derived from the timestamp at read
  time rather than stored

#### Scenario: Sources are combined without silent merging

- **WHEN** the same project id appears in two selected sources
- **THEN** both records are retained and disambiguated by source, and Forge does
  not merge them into one value

### Requirement: Composable, read-only query

Forge SHALL let a caller filter the catalog by tags, languages, profile,
lifecycle, repository, CI state, Compose state and evidence state, order the
result deterministically, and paginate it, without mutating any project,
registry or provider.

#### Scenario: Filters compose

- **WHEN** a caller supplies several filters
- **THEN** predicates combine as AND, a repeated filter value is OR within that
  predicate, and the result is identical regardless of source arrival order

#### Scenario: A query changes nothing

- **WHEN** any catalog query runs through any transport
- **THEN** no registry byte, table row or journal row is written

### Requirement: Explicit empty, stale and unavailable states

Forge SHALL report an empty catalog, a stale observation and an unavailable
source explicitly, and SHALL NOT treat any of them as healthy, absent, or zero.

#### Scenario: An empty catalog is not an error

- **WHEN** no project is registered
- **THEN** the query returns an empty page with exit zero and an explicit empty
  record list

#### Scenario: An unreadable source is named

- **WHEN** a selected source cannot be read
- **THEN** records from the other sources are returned and the unreadable
  source is reported as `unavailable` with its reason, and no placeholder record
  is invented

### Requirement: Machine output is the contract and credentials never leak

Forge SHALL treat JSON and NDJSON as the machine contract and SHALL redact every
credential-shaped value before output, and SHALL NOT search parent directories
or undeclared sources implicitly.

#### Scenario: Table output is not the contract

- **WHEN** a caller selects JSON or NDJSON output
- **THEN** the serialization is stable, versioned and independent of the
  human table layout

#### Scenario: A credential-looking value is redacted

- **WHEN** a source value carries a credential shape
- **THEN** it is redacted through the shared policy before it reaches any output
