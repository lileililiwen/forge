## ADDED Requirements

### Requirement: Application-owned feature evaluation

The feature adapter SHALL evaluate flags through an application-provided evaluator/filter and SHALL not own feature names, rollout persistence, billing plans, or tenant records.

#### Scenario: Tenant-specific flag
- **WHEN** an endpoint requests a feature decision for a tenant
- **THEN** the application evaluator receives the configured context and the endpoint is allowed or rejected without platform-owned rollout state

### Requirement: Explicit endpoint gating

The adapter SHALL provide opt-in endpoint or handler metadata for feature gates and SHALL return a safe, consistent disabled-feature result.

#### Scenario: Disabled endpoint
- **WHEN** the configured feature is disabled
- **THEN** the endpoint does not execute and returns the configured safe status/problem response

### Requirement: Safe HTTP resilience defaults

The resilience adapter SHALL provide bounded timeout, retry, circuit-breaker, and concurrency policies and SHALL not retry non-idempotent methods by default.

#### Scenario: POST transient failure
- **WHEN** an outbound POST receives a transient failure without explicit retry metadata
- **THEN** the pipeline does not automatically replay the POST

### Requirement: Resilience observability

The adapter SHALL emit bounded policy outcome telemetry and SHALL preserve caller cancellation without logging request bodies or authorization values.

#### Scenario: Cancelled HTTP call
- **WHEN** the caller cancels an outbound request
- **THEN** the pipeline stops promptly and records cancellation without converting it to a retryable provider failure
