# Roadmap

The roadmap is organized into four phases. Items marked **Done** are implemented, archived, and have generated capability specs in `openspec/specs/`.

## Phase 1: Foundation — Done

1. `platform-repository-foundation` — solution, four production projects, central MSBuild and package metadata, dependency-direction guardrails.
2. `platform-core-contracts` — `IClock`, `Error`, `Result`, `Result<T>`, `CallerContext`, `IAuditable` in `Platform.Core`.
3. `platform-aspnetcore-foundation` — `AddPlatformAspNetCore`, `UsePlatformAspNetCore`, `MapPlatformEndpoints`, sanitized `ProblemDetails`, correlation middleware/accessor, and health-check helpers in `Platform.AspNetCore`.

## Phase 2: Reusable product contracts — Done

4. `platform-entitlement-contracts` — opaque identifiers, normalized subscription and entitlement snapshots, structured feature-check decisions, replaceable usage-meter interface, and idempotent processed-event store in `Platform.Billing.Contracts`.
5. `platform-testing-toolkit` — `ControllableClock`, `SubscriptionBuilder`, `EntitlementBuilder`, `FakeEntitlementStore`, and `RecordingUsageMeter` in `Platform.Testing`.

## Phase 3: Pilot adoption — Done

6. `platform-extraction-ratelimiting` — `IRateLimiter`, `InMemoryRateLimiter`, `RateLimitDecision`, `RateLimitKey`, `RateLimitPolicies` (documented default catalog), `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, `RateLimitingOptions`, and `AddPlatformRateLimiting` in `Platform.RateLimiting`.
7. `platform-extraction-mailing` — `MailAddress`, `MailAttachment`, `MailMessage`, `MailSendOutcome`, `MailSendResult`, `IMailService`, `MailTemplateId`, `IMailTemplateRenderer<TModel>`, `RenderedMailTemplate`, `MailingOptions`, and `AddPlatformMailing` in `Platform.Mailing`.
8. `platform-extraction-eventing` — `IIntegrationEvent`, `IntegrationEvent`, `IntegrationEventEnvelope`, default `System.Text.Json` serializer/deserializer, `IIntegrationEventHandler<TEvent>`, `IEventBus`, `InProcessEventBus` (bounded `Channel<T>`), `EventingOptions`, and `AddPlatformEventing` / `AddPlatformEventingInProcess` in `Platform.Eventing`.
9. `platform-extraction-idempotency` — `IdempotencyRecord`, `IIdempotencyStore`, `InMemoryIdempotencyStore`, `RequestFingerprint`, `IdempotencyOptions` (with documented metric-name constants), `IdempotencyMetrics`, and `AddPlatformIdempotency` in `Platform.Idempotency`.
10. `platform-extraction-jobs` — `IJobDispatcher`, `IRecurringJobHandler`, `IRecurringJobRegistry`, `IJobTelemetry`, `RecurringJobAttribute`, `RecurringJobDescriptor`, `JobPayload`, `BackgroundJobsOptions`, and `AddPlatformJobs` in `Platform.Jobs`.

## Deferred

- a shared EF Core persistence package;
- a shared Stripe implementation;
- shared invoice, wallet, or tenant-billing workflows;
- migration of all existing projects;
- a separately deployed billing service;
- automatic synchronization of every application to the newest package version;
- application-specific backends for the new framework-neutral packages (Hangfire/Quartz, SendGrid/Mailgun/SMTP, RabbitMQ/Service Bus, Redis/Postgres idempotency, Redis rate-limit).

## Starter-kit complexity gap queue — Done

The comparison with `/home/paul/code/dotnet-starter-kit` identified adoption
friction rather than a need to reproduce the starter architecture. All six
changes are implemented, strictly validated, and archived:

1. `platform-domain-primitives` — framework-neutral entities, aggregates, domain events, money, optional markers, and safe domain errors (`openspec/changes/archive/2026-09-11-platform-domain-primitives/`).
2. `platform-web-composition-contracts` — explicit per-host module composition without global discovery or Mediator/FluentValidation coupling (`openspec/changes/archive/2026-09-11-platform-web-composition-contracts/`).
3. `platform-efcore-migrator-host` — application-owned migration runner with pending/apply operations and optional seed/lock callbacks (`openspec/changes/archive/2026-09-11-platform-efcore-migrator-host/`).
4. `platform-dotnet-template-pack` — installable minimal `dotnet new` template with detached generated source (`openspec/changes/archive/2026-09-11-platform-dotnet-template-pack/`).
5. `platform-adoption-tooling` — explicit-directory read-only diagnostics and package-alignment preview (`openspec/changes/archive/2026-09-11-platform-adoption-tooling/`).
6. `platform-application-sample-matrix` — focused samples proving incremental adoption (`openspec/changes/archive/2026-09-11-platform-application-sample-matrix/`).

The queue does not include React clients, Aspire, Docker/Terraform, product modules,
shared invoices/wallets, or a workspace-wide migration.

## Phase 5: Web edge integrations — Done

The next phase adds opt-in CORS, HTTP resilience, OpenAPI document mapping, and a shared
redaction-safe telemetry layer as small independently adoptable packages. Each item is an
OpenSpec change; implementation still proceeds one active change at a time.

21. `platform-web-edge` — `Platform.Web.Telemetry` (framework-neutral redaction-safe
    telemetry names and option-validation helpers), `Platform.Web.Cors` (named
    CORS policies with production validation), `Platform.Web.Resilience`
    (`HttpClient` retry/timeout/circuit-breaker with bounded defaults and
    idempotent-method handling), and `Platform.Web.OpenApi` (named document
    registry with explicit mapping; the platform owns no Swashbuckle/NSwag
    dependency). **Done**

### Phase 5 dependency order

`Platform.Web.Telemetry` is the framework-neutral root. The three ASP.NET Core
packages (`Cors`, `Resilience`, `OpenApi`) each depend on it; nothing in
`Platform.Web` changes.

### Phase 5 non-goals

- no Swashbuckle, NSwag, Polly, OpenTelemetry exporter, or third-party CORS library;
- no SignalR, SSE, or application module loader;
- no mass filesystem move of existing packages;
- no change to the existing `Platform.Web` runtime surface.

## Phase 4: Application platform roadmap — Done

The next phase turns the current capability packages into a coherent, Spring-Boot-like
application platform for independent demo and development products. Each item is an
independently adoptable OpenSpec change; implementation still proceeds one active change
at a time.

11. `platform-web-runtime-foundation` — shared configuration validation, security headers, observability, request limits, and a consistent web bootstrap. **Done**
12. `platform-persistence-efcore` — optional EF Core persistence conventions, migrations/readiness helpers, auditing, soft-delete, pagination, and specifications. **Done**
13. `platform-identity-authorization` — replaceable identity contracts, current-user context, provider seams, permission catalog, policy helpers, and test doubles. **Done**
14. `platform-admin-capability` — reusable user, role, permission, session, audit, provider-health, and subscription overview contracts/endpoints. **Done**
15. `platform-billing-provider-abstractions` — normalized subscription, checkout, portal, webhook, entitlement, usage, and provider-event orchestration contracts. **Done**
16. `platform-billing-provider-adapters` — optional Stripe and Lemon Squeezy adapters with signature verification, normalization, idempotency, and provider health. **Done**
17. `platform-ai-provider-abstractions` — provider-neutral generation, structured output, streaming, embeddings, usage, cost, policy, safe failures, adapters, and deterministic testing. **Done**
18. `platform-notifications-sms` — notification intent, SMS contracts, provider-neutral delivery results, test adapters, and retry/failure semantics. **Done**
19. `platform-ui-design-system` — shared design tokens, Razor web assets, accessible state components, API client conventions, and admin/dashboard shell contracts. **Done**
20. `platform-application-starter` — composite application bootstrap, project template, sample host, configuration defaults, and adoption guides. **Done**

### Phase 4 dependency order

`platform-web-runtime-foundation` → `platform-persistence-efcore` →
`platform-identity-authorization` → `platform-admin-capability` →
`platform-billing-provider-abstractions` → `platform-billing-provider-adapters`.

`platform-ai-provider-abstractions`, `platform-notifications-sms`, and
`platform-ui-design-system` can begin after the web/runtime contracts are stable.
`platform-application-starter` is intentionally last because it composes the earlier
capabilities and must not become a second monolithic starter-kit architecture.

### Phase 4 non-goals

- no shared product-domain entities, invoice rules, plan names, prompts, or business permissions;
- no mandatory EF Core, PostgreSQL, Redis, Hangfire, Stripe, Lemon Squeezy, OpenAI, Claude, or SMS dependency in the base packages;
- no separately deployed platform service;
- no forced migration of every application;
- no promise that React and Razor share implementation code; they share tokens, contracts, and interaction semantics.

## Status summary

- Active changes: none (`openspec list` is empty); the shared-test-web-factory, platform-build-policy-package, and ui-shared-layer-reachability queue is archived.
- Archived changes: fifty-eight (see `openspec/changes/archive/`).
- Generated specs: fifty-one (see `openspec/specs/`).
- Source projects: 75 (`src/`) plus 52 in-solution test projects (`tests/`), 6 samples (`samples/`), 3 template projects (`templates/`), and 1 CLI tool (`tools/`).
- Tests: 1,391 passing across 48 in-solution test suites under SDK `10.0.400` (`grep` total over `dotnet test Platform.sln -c Release --no-build --no-restore --nologo -m:1`; architecture suite alone is 367 guard tests in `Platform.Architecture.Tests`). The solution-excluded consumer-conformance fixture (`tests/Platform.ConsumerConformance/`) adds the shared-contract envelope validation and the packed-artifact smoke test.
- Baseline: SDK `10.0.400`, all projects `net10.0`, Microsoft framework packages at `10.0.0` (except `Microsoft.Extensions.Caching.Hybrid 9.3.0`, `Microsoft.Extensions.Http.Resilience 8.10.0`, `Microsoft.FeatureManagement 4.5.0`, and ASP.NET Core versioning helpers pinned by compatibility), third-party packages unchanged.
