## ADDED Requirements

### Requirement: Application-owned migration runner
The migrator MUST execute against a `DbContext` and migration assembly supplied by the application and MUST NOT create platform-owned contexts, migrations, connection strings, or seed data.

#### Scenario: Consumer supplies context
- **WHEN** an application supplies a context factory and migration configuration
- **THEN** the runner inspects or applies only that application's migrations

### Requirement: Pending and apply operations
The runner MUST support a read-only pending-migration operation and a separate apply operation, both honoring cancellation.

#### Scenario: Read-only inspection
- **WHEN** the application requests pending migrations
- **THEN** the runner returns the pending identifiers and does not modify the database

### Requirement: Optional seed and exclusive execution seams
The runner MUST accept optional application callbacks for seeding and exclusive execution and MUST not impose a lock or seed policy when callbacks are absent.

#### Scenario: Application controls locking
- **WHEN** an application supplies an exclusive-execution callback
- **THEN** migration and optional seed execution occur inside that callback

### Requirement: Safe failure results
Migration failures MUST expose a stable failure category and secret-free diagnostic information and MUST NOT expose connection strings, SQL, exception messages, or provider response bodies through the result.

#### Scenario: Database unavailable
- **WHEN** the supplied context cannot connect
- **THEN** the result is unsuccessful with a stable unavailable category and redacted diagnostic data
