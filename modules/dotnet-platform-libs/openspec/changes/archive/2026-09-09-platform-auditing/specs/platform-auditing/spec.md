## ADDED Requirements

### Requirement: Normalized audit events

The platform SHALL define immutable audit events carrying action, outcome, severity, occurrence time, correlation, optional tenant/subject, and masked metadata without requiring a persistence provider.

#### Scenario: Security decision event
- **WHEN** an authorization decision is recorded
- **THEN** the event contains normalized action/outcome and safe subject/correlation metadata without credentials

### Requirement: Pluggable sinks and enrichment

The auditing contracts SHALL allow applications to provide sinks, enrichers, masking services, and failure policy without referencing platform persistence entities.

#### Scenario: Custom sink
- **WHEN** an application registers a sink
- **THEN** captured events are delivered to that sink using cancellation and the configured failure policy

### Requirement: HTTP and exception capture

The ASP.NET Core adapter SHALL capture configured request, response, security, and exception events with bounded metadata and no raw secrets or request bodies by default.

#### Scenario: Unhandled exception
- **WHEN** a request ends with an unhandled exception
- **THEN** the adapter emits a normalized exception event with correlation and safe classification

### Requirement: EF change capture

The EF Core adapter SHALL capture changes only for explicitly configured entities/properties and SHALL mask sensitive values before invoking a sink.

#### Scenario: Sensitive entity update
- **WHEN** an opted-in entity changes a password-like property
- **THEN** the audit diff contains the property change classification but not the old or new secret value
