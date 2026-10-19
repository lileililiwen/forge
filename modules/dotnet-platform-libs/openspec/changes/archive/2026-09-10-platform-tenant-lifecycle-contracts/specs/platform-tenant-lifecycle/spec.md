## ADDED Requirements

### Requirement: Tenant lifecycle SHALL be application-owned

The platform SHALL orchestrate tenant lifecycle steps through application-provided catalog, persistence, migration, and seed callbacks and SHALL not define tenant entities, migrations, connection strings, plans, or seed records.

#### Scenario: Consumer registers a provisioning workflow
- **WHEN** an application supplies tenant callbacks and ordered steps
- **THEN** the platform executes the workflow without referencing an application DbContext or tenant entity type

### Requirement: Provisioning SHALL be idempotent and resumable

Each lifecycle step SHALL have a stable name and idempotency boundary, and a retry SHALL resume from recorded state without reapplying completed steps.

#### Scenario: Retry after a transient failure
- **WHEN** step two fails transiently after step one completed
- **THEN** a retry skips step one and retries step two using the same lifecycle operation identity

### Requirement: Failures SHALL be classified

Lifecycle steps SHALL classify failures as retryable, permanent, canceled, or policy-denied and SHALL expose safe status without secrets or provider exception text.

#### Scenario: Permanent migration configuration failure
- **WHEN** an application callback reports an invalid migration configuration
- **THEN** the operation becomes permanently failed and operator status contains a stable safe code

### Requirement: Tenant context SHALL be isolated

The orchestrator SHALL establish and restore tenant scope around each tenant-scoped step and SHALL not leak a tenant scope into another operation.

#### Scenario: Sequential tenant provisioning
- **WHEN** two tenants are provisioned sequentially in one worker
- **THEN** each step observes only its own tenant scope and the worker ends with no ambient tenant
