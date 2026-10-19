## ADDED Requirements

### Requirement: Cache operations are asynchronous and provider-neutral
The platform SHALL provide asynchronous cache get, set, remove, and invalidation contracts without requiring a cache provider or serializer in the contract package.

#### Scenario: Application supplies its own serializer
- **WHEN** an application registers a cache adapter
- **THEN** the application can choose serialization and storage without changing the platform contract

### Requirement: Cache keys are validated and isolated
The platform SHALL provide key construction that rejects invalid keys and supports explicit application and tenant prefixes.

#### Scenario: Two tenants use the same logical key
- **WHEN** the key builder receives different tenant identifiers
- **THEN** it produces different physical keys

### Requirement: Expiration and invalidation are explicit
The platform SHALL support absolute expiration and SHALL expose removal by key and by portable tag where the adapter supports tags.

#### Scenario: Tagged entries are invalidated
- **WHEN** an application invalidates a tag
- **THEN** subsequent reads do not return entries associated with that tag

### Requirement: Provider failures have safe behavior
The platform SHALL classify cache provider failures without exposing connection strings or payloads and SHALL allow adapters to return a cache miss for transient read failures.

#### Scenario: Distributed cache is unavailable
- **WHEN** a transient provider failure occurs during a read
- **THEN** the adapter returns a safe miss and exposes unavailable provider status

### Requirement: Cache behavior is observable
The platform SHALL define stable operation names for cache reads, writes, removals, hits, misses, and failures without recording raw keys or values by default.

#### Scenario: A cache miss is recorded
- **WHEN** a factory populates a missing entry
- **THEN** telemetry records a miss and does not include the raw cache value
