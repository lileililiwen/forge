## ADDED Requirements

### Requirement: Canonical one-way distribution

Forge SHALL implement forge mirror to preserve a configured canonical primary and synchronize selected refs one-way to enabled mirrors, initially GitHub to Gitee.

#### Scenario: Canonical one-way distribution success

- **WHEN** an authorized distribution has a valid primary and mirror
- **THEN** the selected primary refs are copied and both outcomes are recorded

#### Scenario: Canonical one-way distribution failure

- **WHEN** a mirror has divergent protected history
- **THEN** synchronization blocks that update without force or reverse-syncing into the primary

#### Scenario: Canonical one-way distribution boundary

- **WHEN** a mirror is disabled
- **THEN** no write is attempted to that remote

### Requirement: Partial failure and credentials

Forge SHALL report per-remote outcomes and support retrying failed mirror delivery without misreporting primary state or exposing credentials.

#### Scenario: Partial failure and credentials success

- **WHEN** primary delivery succeeds but Gitee is unavailable
- **THEN** the operation records primary success and mirror failure separately

#### Scenario: Partial failure and credentials failure

- **WHEN** credentials are invalid
- **THEN** the operation reports authentication failure with secrets redacted

#### Scenario: Partial failure and credentials boundary

- **WHEN** the failed mirror is retried after recovery
- **THEN** already delivered refs remain intact and only the requested outstanding work executes
