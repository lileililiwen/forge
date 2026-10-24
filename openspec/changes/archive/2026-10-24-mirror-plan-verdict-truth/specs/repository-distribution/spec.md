## MODIFIED Requirements

### Requirement: Partial failure and credentials

Forge SHALL report per-remote outcomes and support retrying failed mirror delivery without misreporting primary state or exposing credentials. The report SHALL carry a verdict that names what the run did, so a partial external write, a plan that attempted no write and a delivery are never reported as the same thing.

#### Scenario: Partial failure and credentials success

- **WHEN** primary delivery succeeds but Gitee is unavailable
- **THEN** the operation records primary success and mirror failure separately

#### Scenario: Partial failure and credentials failure

- **WHEN** credentials are invalid
- **THEN** the operation reports authentication failure with secrets redacted

#### Scenario: Partial failure and credentials boundary

- **WHEN** the failed mirror is retried after recovery
- **THEN** already delivered refs remain intact and only the requested outstanding work executes

#### Scenario: A plan never claims a delivery

- **WHEN** a mirror run is planned instead of applied
- **THEN** the report's verdict names the run a plan and the per-remote outcomes still name the provider that is unavailable
