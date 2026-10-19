## ADDED Requirements

### Requirement: Hangfire job dispatch

The adapter SHALL implement platform job dispatch and recurring registration using Hangfire without changing the engine-neutral `Platform.Jobs` API.

#### Scenario: Enqueue payload
- **WHEN** a consumer dispatches a valid platform job payload
- **THEN** the adapter enqueues a Hangfire job with the payload and returns the platform dispatch result

### Requirement: Scoped execution context

The adapter SHALL create a scoped execution context for each job and SHALL restore the application-provided tenant and subject context before resolving the handler.

#### Scenario: Tenant job
- **WHEN** a job carries tenant and subject metadata
- **THEN** the handler resolves inside that tenant/subject scope and the scope is disposed after execution

### Requirement: Protected dashboard

The adapter SHALL keep dashboard mapping opt-in and SHALL require an application-provided authorization callback before serving it.

#### Scenario: Unauthorized dashboard request
- **WHEN** a request fails the configured dashboard authorization callback
- **THEN** the dashboard denies access without exposing job storage details

### Requirement: Job provider health

The adapter SHALL expose safe provider availability and failure information through the platform provider-status seam without returning connection strings or credentials.

#### Scenario: Storage unavailable
- **WHEN** Hangfire storage cannot be reached
- **THEN** readiness reports the provider unavailable with a redacted diagnostic
