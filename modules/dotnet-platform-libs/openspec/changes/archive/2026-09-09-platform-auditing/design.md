## Context

Starter references include the auditing contracts, HTTP scope, entity diff builder, masking service, SQL/file sinks, retention job, and security/exception event capture.

Starter-kit references:

- `dotnet-starter-kit/src/Modules/Auditing/Modules.Auditing.Contracts/`
- `dotnet-starter-kit/src/Modules/Auditing/Modules.Auditing/Core/DefaultAuditScope.cs`
- `dotnet-starter-kit/src/Modules/Auditing/Modules.Auditing/Persistence/EntityDiffBuilder.cs`
- `dotnet-starter-kit/src/Modules/Auditing/Modules.Auditing/Persistence/AuditingSaveChangesInterceptor.cs`
- `dotnet-starter-kit/src/Modules/Auditing/Modules.Auditing/Persistence/AuditRetentionJob.cs`

Agents may copy event-shape and masking tests as reference, but must remove starter module DTOs, product endpoint assumptions, and persistence ownership.

## Goals / Non-Goals

**Goals:**

- Define a normalized audit event contract with correlation, tenant, subject, action, outcome, severity, and safe metadata.
- Provide composable HTTP, exception, security, and EF change capture adapters.
- Support application-owned sinks, masking, enrichment, retention, and dead-letter handling.

**Non-Goals:**

- Owning an audit database, migration, query API, retention schedule, product permission catalog, or business event schema.
- Guaranteeing that auditing can never fail a business request; failure policy must be explicit.
- Capturing raw request bodies, passwords, tokens, or unrestricted entity snapshots.

## Decisions

1. Create `Platform.Auditing.Contracts`, then separate `Platform.Auditing.AspNetCore` and `Platform.Auditing.EfCore` adapters.
2. Use immutable normalized events and application-provided `IAuditSink`, `IAuditMasker`, and `IAuditEnricher` contracts.
3. Default to fail-open for request processing with a provider-status/dead-letter signal, while allowing applications to choose fail-closed for selected security events.
4. Capture entity diffs only for explicitly opted-in entities/properties and apply masking before serialization.

Alternative rejected: copying the starter's full auditing module would impose its schema, endpoints, and retention jobs on every consumer.

## Risks / Trade-offs

- [Risk] Audit data itself contains secrets → mask before persistence and add tests for common secret names/types.
- [Risk] High-volume HTTP auditing affects latency → bounded async publisher and explicit drop/dead-letter metrics.
- [Risk] EF diff behavior varies by provider → keep capture provider-neutral and test metadata/state transitions independently.

## Migration Plan

Start with recording sink tests, enable HTTP/security capture, then EF capture for one aggregate. Keep the starter auditing module in place until event parity is verified. Rollback disables adapters; application audit storage remains intact.

## Open Questions

- Whether a durable audit outbox belongs in this repository or should remain an application-specific sink implementation.
