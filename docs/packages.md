# Package reference

This document summarizes the public surface of each platform package. It is the consumer-facing companion to `docs/build-test-pack.md`.

## Platform.Core

Framework-independent contracts. Zero third-party dependencies; targets `net10.0`.

### Time

- `IClock` — abstraction for the current UTC time. Implementations must return a `DateTimeOffset` with `Offset == TimeSpan.Zero`.
- `SystemClock` — production implementation backed by `DateTimeOffset.UtcNow`.
- `FixedClock` — deterministic implementation that takes a `DateTimeOffset` or `Func<DateTimeOffset>` at construction.

### Results

- `Error` — immutable record with stable `Code`, safe `Message`, and optional `Metadata`. Helpers: `Error.Validation`, `Error.NotFound`; constants `Error.ValidationCode`, `Error.NotFoundCode`.
- `Result` — non-generic outcome. `Result.Success()`, `Result.Failure(Error)`.
- `Result<T>` — generic outcome. `Result<T>.Success(T)`, `Result<T>.Failure(Error)`, `ToResult()`.

### Context

- `CallerContext` — immutable record with optional `SubjectId` and `TenantId`. Helpers: `IsAnonymous`, `HasTenant`. Static singleton: `CallerContext.Anonymous`.

### Audit

- `IAuditable` — exposes `CreatedAt`, `CreatedBy`, `UpdatedAt`, `UpdatedBy`. Product types implement it without inheriting from a platform base class.

## Platform.AspNetCore

ASP.NET Core integration. Depends on `Platform.Core` and `Microsoft.AspNetCore.App` (via `FrameworkReference`); no third-party packages.

### Registration

- `AddPlatformAspNetCore(IServiceCollection)` and `AddPlatformAspNetCore(IServiceCollection, Action<PlatformAspNetCoreOptions>)` — register `IClock` (`SystemClock`), `IProblemDetailsMapper`, `IHttpContextAccessor`, `ICorrelationAccessor`, and `PlatformAspNetCoreOptions`. No hidden authentication, persistence, or provider services are added.
- `AddPlatformHealthChecks(IServiceCollection)` — adds a `platform.liveness` check tagged `live`. Returns the `IHealthChecksBuilder` for further configuration.

### Pipeline

The documented middleware order is:

1. `UsePlatformCorrelation` — establish or accept a correlation identifier and echo it on the response.
2. `UsePlatformProblemDetails` — translate `PlatformProblemException` to sanitized `ProblemDetails`; sanitize unknown exceptions to a generic `500` with no internal details.
3. `MapPlatformEndpoints` — map the health endpoint at `PlatformAspNetCoreOptions.HealthCheckPath` (default `/health`).

`UsePlatformAspNetCore` runs steps 1 and 2 in the documented order.

### Errors

- `IProblemDetailsMapper` — maps an `Error` to a `ProblemDetails` with status code, type URI, title, detail, and the `code` extension.
- `PlatformProblemDetailsMapper` — default implementation. `platform.validation` → 400, `platform.not_found` → 404, unknown → 500.
- `PlatformProblemException` — exception type that carries an `Error` to the boundary.
- `ProblemDetailsExceptionMiddleware` — converts `PlatformProblemException` to `ProblemDetails`; sanitizes unknown exceptions to a generic 500.

### Correlation

- `ICorrelationAccessor` — exposes the current request's correlation identifier.
- `HttpCorrelationAccessor` — `HttpContext`-backed implementation.
- `CorrelationMiddleware` — reads or generates the correlation identifier and stores it on `HttpContext.Items`; echoes it on the response via `OnStarting`.

### Options

- `PlatformAspNetCoreOptions` — `CorrelationHeader` (default `X-Correlation-Id`), `AcceptIncomingCorrelationHeader` (default `false`), `HealthCheckPath` (default `/health`).

## Platform.Billing.Contracts

Provider-neutral subscription and entitlement contracts. Zero third-party dependencies; targets `net10.0`.

### Identifiers

- `PlanId`, `FeatureKey`, `SubjectKey` (with `Anonymous` and `IsAnonymous`), `ProviderName`, `ProviderEventId` — opaque strongly-typed identifiers. All have a `Create(string)` factory that rejects null/empty/whitespace values.

### Subscriptions

- `SubscriptionStatus` enum — `Free`, `Active`, `Suspended`, `PastDue`, `Canceled`, `Unknown`. Only `Free` and `Active` are treated as active.
- `Subscription` — record carrying `Subject`, `Plan`, `Status`, `Provider`, `ProviderSubscriptionId`, `PeriodStart`, `PeriodEnd`. Helpers: `IsActive`, `IsWithinPeriod(now)`.

### Entitlements

- `Entitlement` — immutable snapshot: `Subject`, optional `Tenant`, optional `Subscription`, `IReadOnlySet<FeatureKey> ActiveFeatures`, optional `IReadOnlyDictionary<FeatureKey, long> Limits`, `CapturedAt`. Helpers: `Grants(FeatureKey)`, `LimitFor(FeatureKey)`.
- `EntitlementDefaults` — static factory: `Anonymous`, `Inactive(subject)`, `Unknown(subject, subscription)`. Unknown subscriptions are preserved for diagnostics but grant no features.

### Features

- `FeatureCheckReason` enum — `Allowed`, `NotAuthenticated`, `NotSubscribed`, `PlanMismatch`, `LimitExceeded`, `Unknown`.
- `FeatureCheckResult` — structured decision: `Feature`, `Reason`, optional `RequiredPlan`/`CurrentUsage`/`Limit`. Helper: `IsAllowed`.
- `FeatureCheck.Evaluate(Entitlement, FeatureKey)` — maps a snapshot into a decision.

### Usage

- `IUsageMeter` — `CheckAsync(subject, feature, cancellationToken)`, `RecordAsync(subject, feature, units, cancellationToken)`. The platform does not prescribe storage, cache, or counting implementation.
- `UsageCheckResult` — `Feature`, `Used`, optional `Limit`, optional `WindowStart`/`WindowEnd`. Helper: `IsWithinLimit`.

### Events

- `ProviderEvent` — opaque provider event: `Id`, `Provider`, `Type`, `OccurredAt`, `Payload`.
- `ProcessedEvent` — stored record: `EventId`, `Provider`, `ProcessedAt`, `Result`.
- `ProcessedEventDecision` enum — `FirstDelivery`, `Duplicate`.
- `IProcessedEventStore` — `MarkProcessedAsync(processedEvent, cancellationToken)` returns the idempotency decision.

## Platform.Billing provider adapters

- `Platform.Billing.Stripe` — optional raw-HTTP Stripe adapter with application-owned plan
  mapping, checkout, portal, subscription lookup, webhook verification/normalization, and status.
- `Platform.Billing.LemonSqueezy` — optional raw-HTTP Lemon Squeezy adapter with checkout,
  subscription lookup, webhook verification/normalization, and status. Its unsupported portal
  and cancellation operations are explicit.
- `ProviderFailureClassifier` and `ProviderFailure` — safe transient, permanent, configuration,
  authentication, and malformed-response categories without secrets or response bodies.

## Platform.Billing

`Platform.Billing` is the application-owned billing seam. It depends on
`Platform.Billing.Contracts` and `Platform.Core`; targets `net10.0`. The
package owns the `IBillingProvider` interface, the application-supplied
checkout/portal mapping, the webhook normalization, and the provider
status surface. The provider-specific adapters (`Platform.Billing.Stripe`,
`Platform.Billing.LemonSqueezy`) implement `IBillingProvider`; consumers
register exactly one. The plan, price, invoice, and product catalog stay
in the consumer.

## Platform.Billing.Testing

Deterministic billing test doubles. Depends on `Platform.Billing.Contracts`
and `Platform.Core`; targets `net10.0`. No xUnit, NUnit, or
mocking-framework dependencies. Production projects must not reference
this package.

- `SubscriptionBuilder` — fluent builder for `Subscription`. Defaults to
  active status, `plan.test`, `test` provider, `sub_test` provider id,
  and a 30-day period anchored at the supplied `IClock`. `WithPlan`,
  `WithStatus`, `WithProvider`, `WithProviderSubscriptionId`,
  `WithPeriodStart`, `WithPeriodEnd`.
- `EntitlementBuilder` — fluent builder for `Entitlement`. Safe inactive
  defaults (no features, no limits). `WithTenant`, `WithSubscription`,
  `Granting(feature)`, `Granting(IEnumerable<FeatureKey>)`,
  `WithLimit(feature, limit)`, `CapturedAt`.
- `FakeEntitlementStore` — in-memory store with `Configure`, `Get`,
  `Invalidate`, `InvalidatedSubjects`, `Reset`. `Get` returns
  `EntitlementDefaults.Inactive` for unknown or invalidated subjects.
- `RecordingUsageMeter` — `IUsageMeter` that records every call and
  accumulates totals per `(subject, feature)`. `SetLimit(feature, limit?)`
  configures per-feature limits; `TotalFor`; `Calls`; `Reset` clears
  totals and calls but preserves limits.

## Platform.Ai

`Platform.Ai.Contracts` provides provider-neutral generation, streaming, structured-output,
embedding, usage, cost, capability, policy, and failure contracts without provider references.
`Platform.Ai` provides policy-gated generation and single- or feature-based routing with safe
telemetry. `Platform.Ai.Testing` provides deterministic fakes and usage recording.

The optional `Platform.Ai.OpenAiCompatible`, `Platform.Ai.Anthropic`, and `Platform.Ai.Ollama`
packages use raw HTTP only. OpenAI-compatible configuration also supports DeepSeek endpoint
selection. See [`docs/platform-ai.md`](platform-ai.md) for data-handling and adoption rules.

## Platform.Jobs

Engine-neutral scheduling contract. Depends on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, Hangfire, Quartz, or application projects.

### Contracts

- `IJobDispatcher` — `EnqueueAsync(payload, cancellationToken)`. Implementations read the current time from an injected `IClock`; the platform exposes only the contract.
- `IRecurringJobHandler` — marker interface; `ExecuteAsync(cancellationToken)`.
- `IRecurringJobRegistry` — `Register(descriptor)`, `Registered` enumeration. Implementations invoke `IJobTelemetry` on every register.
- `IJobTelemetry` — `JobRegistered(descriptor, registeredAt)`, `JobEnqueued(payload, enqueuedAt)`, `JobExecuted(name, executedAt)`, `JobFailed(name, error, failedAt)`. The platform does not ship a default implementation.

### Value types

- `JobPayload` — record with `Name` and optional `Arguments` dictionary. `Create(name, arguments)` factory rejects null/empty/whitespace names.
- `RecurringJobDescriptor` — record with `Name`, `Cron`, `HandlerType`, `TimeZone` (default `"UTC"`), optional `Options` dictionary. `WithName`/`WithCron`/`WithOptions` helpers return a new descriptor.
- `RecurringJobAttribute` — runtime attribute (target: `Class`, `AllowMultiple = false`, `Inherited = false`) with a required `Cron` constructor argument and optional `Name`, `TimeZone` (default `"UTC"`), `Options`. The static `GetDescriptor(Type)` helper returns a descriptor for a decorated handler and throws `InvalidOperationException` for an undecorated one.

### Options

- `BackgroundJobsOptions` — `DefaultTimeZone` (default `"UTC"`), `SectionName` constant `"BackgroundJobs"`. Bound by `AddPlatformJobs` through the `IOptions<>` pipeline.

### Registration

- `AddPlatformJobs(IServiceCollection)` and `AddPlatformJobs(IServiceCollection, Action<BackgroundJobsOptions>)` — bind `BackgroundJobsOptions` and register `IClock` only when no implementation is already present. The package does not register default `IJobDispatcher`, `IRecurringJobRegistry`, or `IJobTelemetry`; consumers provide their own.

## Platform.Jobs.Hangfire

Optional Hangfire adapter for the platform scheduling contracts. Depends on `Platform.Jobs`, Hangfire (`Hangfire.Core`, `Hangfire.AspNetCore`, `Hangfire.InMemory`, `Hangfire.PostgreSql`), `Microsoft.Extensions.Options`, `Microsoft.Extensions.DependencyInjection.Abstractions`, and `Microsoft.Extensions.Diagnostics.HealthChecks.Abstractions`; targets `net10.0` and declares the `Microsoft.AspNetCore.App` framework reference for the opt-in dashboard. Does not reference EF Core, Quartz, Redis, Stripe, Npgsql directly, or application projects.

### Seams

- `IJobExecutionContext` — application-owned bridge: `Capture()` returns a `JobContextSnapshot` (opaque `TenantId`/`SubjectId` only) when a context is active, `Restore(snapshot)` returns a disposable that undoes the restoration. No default is registered; a job that carries a captured context without a registered bridge fails closed.
- `IJobPayloadHandler` — application-owned handler invoked for every dispatched payload inside the restored context.
- `HangfireJobsOptions.DashboardAuthorization` — application-provided `Func<DashboardContext, bool>`; required when the dashboard is enabled. The platform ships no credentials or default policy.

### Adapter

- `HangfireJobDispatcher` (`IJobDispatcher`) — serializes the payload (System.Text.Json, web defaults) and enqueues it through the Hangfire client into the configured queue; records `IJobTelemetry.JobEnqueued` and honours caller cancellation.
- `HangfireRecurringJobRegistry` (`IRecurringJobRegistry`) — attaches the platform recurring executor to the descriptor's cron expression and time zone through `IRecurringJobManager`; first registration wins, later registrations with the same name are no-ops; records `IJobTelemetry.JobRegistered`.
- `HangfireJobExecutor` — the Hangfire-invoked entry point. Dispatched payloads are deserialized (argument values round-trip as JSON values; complex values arrive as `JsonElement`) and routed to `IJobPayloadHandler`; recurring executions resolve the descriptor from the registry and invoke the registered `IRecurringJobHandler`. Successes record `JobExecuted`; failures record `JobFailed` with the stable `jobs.execution_failed` code, a fixed safe message, and only the exception type name, then rethrow so Hangfire's automatic retry model stays the retry owner. Cancellation is rethrown and never recorded as a failure.
- `JobContextCaptureFilter` (client filter) — captures the ambient context through `IJobExecutionContext` at job creation and stores it as a Hangfire job parameter.
- `ScopedJobActivator` — creates a DI scope per job execution, restores the captured context before any handler is resolved, and disposes the restoration and scope when the job ends.
- `HangfireStorageHealthCheck` (`IHealthCheck`) — probes storage reachability; reports healthy/unavailable with a redacted diagnostic (exception type only — never messages, connection strings, or storage responses) and exposes `HangfireJobsProviderStatus`.
- `HangfireDashboardOptionsFactory` — builds dashboard options that never display the storage connection string and gate every request through the application callback.

### Options

- `HangfireJobsOptions` (`SectionName` `"BackgroundJobs:Hangfire"`) — `Storage` (`InMemory` default, `PostgreSql`), `PostgreSqlConnectionString` (application-supplied, required for PostgreSql, never echoed), `Queue` (`"default"`), `Queues` (1–20 names), `WorkerCount` (1–100, default 5), `SchedulePollingInterval`/`HeartbeatInterval` (1s–10min, default 30s), `DashboardEnabled` (default `false`), `DashboardRoute` (`"/jobs"`), `DashboardAuthorization`. `Validate()` is invoked at registration with secret-free messages.

### Registration

- `AddPlatformHangfireJobs(IServiceCollection, Action<HangfireJobsOptions>?)` — validates the options, wires the application-selected storage, the scoped activator, the capture filter, and the Hangfire server, then `TryAdd`s `HangfireJobExecutor`, `IJobDispatcher`, `IRecurringJobRegistry`, `IHealthCheck`, and `IClock`. Registration is idempotent; application-owned dispatcher/registry registrations win.
- `AddPlatformHangfireJobs(IServiceCollection, Action<HangfireJobsOptions>?, JobStorage)` — same as above but uses an application-owned `JobStorage` instance, which the service provider registers as a DI singleton so the storage lifetime is owned by the host (used by tests for isolated lifetime and disposal).
- `UsePlatformHangfireDashboard(IApplicationBuilder)` — maps the dashboard only when `DashboardEnabled` is `true` and fails fast with `InvalidOperationException` when the authorization callback is missing.

## Platform.Mailing

Provider-neutral mailing contract. Depends on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, SendGrid, Mailgun, SMTP, Razor, Liquid, or application projects.

### Value types

- `MailAddress` — record with `Address` and optional `DisplayName`. `Create(address)` factory rejects null/empty/whitespace.
- `MailAttachment` — record with `FileName`, `ContentType`, `Content` bytes. `Create(fileName, contentType, content)` factory validates all three.
- `MailMessage` — sealed record with a validating primary constructor. At least one of `TextBody` or `HtmlBody` must be supplied; `Subject` must be non-empty; at least one recipient is required; `From` must be non-null. Optional `Attachments` (defaults to empty) and `CorrelationId`.
- `MailSendOutcome` — enum: `Sent`, `TransientFailure`, `PermanentFailure`, `Bounced`.
- `MailSendResult` — record with `Outcome`, optional `ProviderMessageId`, `ErrorCode`, `ErrorMessage`. `IsAccepted` is `true` for `Sent` or `Bounced`.
- `MailTemplateId` — `readonly record struct` with a validating constructor and implicit conversions to and from `string` (the conversion from `string` re-validates).
- `RenderedMailTemplate` — record with `Subject`, optional `TextBody`, optional `HtmlBody`. `Create(subject, textBody, htmlBody)` factory enforces the subject non-empty and body invariants.

### Contracts

- `IMailService` — `SendAsync(message, cancellationToken)`. Implementations return a `MailSendResult`.
- `IMailTemplateRenderer<TModel>` — `RenderAsync(templateId, model, cancellationToken)`. The platform does not ship a default renderer.

### Options

- `MailingOptions` — `DefaultFromAddress` (default `"noreply@example.invalid"`), `DefaultFromDisplayName` (default `"Platform"`), `MaxAttempts` (default `3`), `InitialBackoffSeconds` (default `5`), `MaxBackoffSeconds` (default `60`), `TemplatesPath` (default `"templates"`), `SectionName` constant `"Mailing"`.

### Registration

- `AddPlatformMailing(IServiceCollection)` and `AddPlatformMailing(IServiceCollection, Action<MailingOptions>)` — bind `MailingOptions` and register `IClock` only when no implementation is already present. The package does not register default `IMailService` or `IMailTemplateRenderer<TModel>`; consumers provide their own.

## Platform.Mailing.Smtp

Optional SMTP adapter over `IMailService` built on MailKit. Depends on `Platform.Mailing`, MailKit, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, SendGrid, or application projects.

### Value types

- `SmtpSecureMode` — enum: `StartTls` (default), `SslOnConnect`, `None`. No auto-negotiation; ambiguous configurations are rejected by making the mode explicit.
- `SmtpMailProviderState` — enum: `Healthy`, `Unavailable`.
- `SmtpMailProviderStatus` — record with `Provider` (constant `"smtp"`), `State`, and optional `LastErrorCode`. Never carries server responses or credentials.

### Options

- `SmtpMailOptions` — `Host` (required), `Port` (default `587`, 1–65535), `SecureMode`, `UserName`/`Password` (configured together or not at all; never written to diagnostics), `OperationTimeout` (default 30s, bounded 1s–5min), `ClientFactory` (optional `Func<SmtpClient>` seam), `SectionName` constant `"Mailing:Smtp"`. `Validate()` is invoked at registration and construction.

### Adapter

- `SmtpMailService` — implements `IMailService`. Builds the MIME message from `MailMessage` (sender, recipients, subject, text/HTML parts, attachments with parsed content types, `X-Correlation-Id` header), connects with the configured secure mode, authenticates when credentials are configured, and sends within the bounded operation timeout. Outcomes are normalized: accepted → `Sent`; SMTP 4xx, connection, TLS, and timeout failures → `TransientFailure`; SMTP 5xx and authentication failures → `PermanentFailure`. Caller cancellation is rethrown, never converted into a provider failure. Invalid sender/recipient/attachment input returns a configuration failure (`mail.configuration.*`) without contacting the server. Diagnostics carry stable error codes and fixed safe messages only; provider responses are never surfaced. Exposes `Status` (`SmtpMailProviderStatus`).

### Registration

- `AddPlatformSmtpMail(IServiceCollection, Action<SmtpMailOptions>?)` — validates the configuration at registration and `TryAdd`s `IMailService` → `SmtpMailService`, so an application-owned `IMailService` registration always wins.

## Platform.Mailing.SendGrid

Optional SendGrid adapter over `IMailService` built on the SendGrid client. Depends on `Platform.Mailing`, the SendGrid client, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, MailKit, or application projects.

### Value types

- `SendGridMailProviderState` — enum: `Healthy`, `Unavailable`.
- `SendGridMailProviderStatus` — record with `Provider` (constant `"sendgrid"`), `State`, and optional `LastErrorCode`. Never carries response bodies or the API key.

### Options

- `SendGridMailOptions` — `ApiKey` (required; never written to diagnostics), `OperationTimeout` (default 30s, bounded 1s–5min), `SectionName` constant `"Mailing:SendGrid"`. `Validate()` is invoked at registration and construction.

### Adapter

- `SendGridMailService` — implements `IMailService` over an application-owned `ISendGridClient`. Maps sender, recipients, subject, plain-text/HTML bodies, base64 attachments, and the `X-Correlation-Id` global header; sends within the bounded operation timeout. Response classification: 2xx → `Sent` (with `X-Message-Id` as `ProviderMessageId` when present); 429 → `TransientFailure` (`mail.sendgrid.rate_limited`); 5xx → `TransientFailure` (`mail.sendgrid.server_error`); other 4xx → `PermanentFailure` (`mail.sendgrid.rejected`); transport and timeout failures → `TransientFailure`. Response bodies are never surfaced. Invalid sender/recipient/attachment input returns a configuration failure (`mail.configuration.*`) without calling the provider. Exposes `Status` (`SendGridMailProviderStatus`).

### Registration

- `AddPlatformSendGridMail(IServiceCollection, Action<SendGridMailOptions>?)` — validates the configuration at registration and `TryAdd`s both `ISendGridClient` (default `SendGridClient` with `HttpErrorAsException = false`) and `IMailService` → `SendGridMailService`, so application-owned registrations always win.

## Platform.Eventing

Transport-agnostic event-bus contract. Depends on `Platform.Core`, `Microsoft.Extensions.Options`, `Microsoft.Extensions.DependencyInjection.Abstractions`, and `Microsoft.Extensions.Logging.Abstractions`; targets `net10.0`. `System.Threading.Channels` ships in-box with `net10.0`. Does not reference ASP.NET Core, EF Core, RabbitMQ, or application projects.

### Value types

- `IIntegrationEvent` — marker interface for typed events.
- `IntegrationEvent` — `abstract record` with `EventId`, `OccurredAt`, `CorrelationId`. Two protected convenience constructors generate a fresh `EventId`.
- `IntegrationEventEnvelope` — transport-agnostic wire format with `MessageId`, `PayloadType`, `PayloadJson`, `OccurredAt`, optional `CorrelationId`.

### Contracts

- `IEventBus` — `PublishAsync(envelope, cancellationToken)`.
- `IIntegrationEventEnvelopeSerializer` — `Serialize(event, cancellationToken)`. Default `IntegrationEventEnvelopeSerializer` is `System.Text.Json`-backed, reads the current time from `IClock` when the event does not already carry a non-default `OccurredAt`, and defaults to camelCase JSON options.
- `IIntegrationEventEnvelopeDeserializer` — `Deserialize(envelope, cancellationToken)`. Default `IntegrationEventEnvelopeDeserializer` resolves the payload type from `PayloadType` (via `Type.GetType` plus a fallback scan of loaded assemblies) and reconstructs the typed event with the envelope's `OccurredAt` and `CorrelationId`.
- `IIntegrationEventHandler<TEvent>` — `ConsumerName` plus `HandleAsync(payload, envelope, cancellationToken)`.

### Default adapter

- `InProcessEventBus` — `IEventBus` + `IAsyncDisposable` with a bounded `Channel<IntegrationEventEnvelope>` (default capacity 1024 from `EventingOptions.InProcessBoundedCapacity`). Back-pressures the publisher through `ChannelWriter.WaitToWriteAsync`, consumes `IClock` and the registered deserializer, resolves every `IIntegrationEventHandler<TEvent>` whose `TEvent` matches the deserialised payload's runtime type from the host's `IServiceProvider`, and logs+swallows consumer failures so the bus never crashes. Disposal is idempotent.

### Options

- `EventingOptions` — `InProcessBoundedCapacity` (default `1024`), `SectionName` constant `"Eventing"`.

### Registration

- `AddPlatformEventing(IServiceCollection)` and `AddPlatformEventing(IServiceCollection, Action<EventingOptions>)` — bind `EventingOptions` and register `IIntegrationEventEnvelopeSerializer` and `IIntegrationEventEnvelopeDeserializer`. Also call `TryAddSingleton<IClock>(_ => new SystemClock())` so the package works when no host clock is registered.

## Platform.Eventing.Contracts and Platform.Eventing.EfCore

`Platform.Eventing.Contracts` provides framework-neutral durable outbox/inbox
records, lease and retry states, store interfaces, `IDurableEventPublisher`,
and thread-safe in-memory stores. `Platform.Eventing.EfCore` is optional and
maps those records into an application-owned EF Core context; it does not own
migrations or select a transport. See
[`platform-eventing-durable.md`](platform-eventing-durable.md).
- `AddPlatformEventingInProcess(IServiceCollection)` — additionally registers `InProcessEventBus` and binds it to `IEventBus`.

## Platform.Eventing.RabbitMq

Optional RabbitMQ adapter over the durable eventing contracts. Depends on
`Platform.Eventing.Contracts`, `RabbitMQ.Client`, and the `Microsoft.Extensions.*`
abstractions; targets `net10.0`. Does not reference ASP.NET Core, EF Core, or
application projects. See [`platform-eventing-rabbitmq.md`](platform-eventing-rabbitmq.md).

- `RabbitMqDurableEventPublisher` — `IDurableEventPublisher` + `IAsyncDisposable` that resolves the envelope's payload type through the application-owned topology, publishes persistent `application/json` messages with `MessageId`/correlation/timestamp and `payload-type`/`tenant-id` headers, and completes only after a publisher confirmation. Reuses an open channel, re-creates closed or failed channels, and bounds every connect and confirmation wait.
- `RabbitMqEventingOptions` — bounded connection, exchange, exchange declaration, and timeout configuration (`SectionName` `"Eventing:RabbitMq"`); validated at registration and construction with secret-free messages.
- `IRabbitMqEventTopology` / `RabbitMqEventTopology` / `RabbitMqEventBinding` — application-owned payload-type-to-routing-key registration with an optional exchange override; the fail-closed `UnconfiguredRabbitMqEventTopology` default rejects every publish.
- `IRabbitMqChannelFactory` / `IRabbitMqChannel` — transport seams (default `RabbitMqChannelFactory`/`RabbitMqChannel` over the RabbitMQ client) so applications can share connections or supply an in-process test transport.
- `RabbitMqPublishException` / `RabbitMqPublishFailure` — the only surfaced failure shape: stable code, fixed safe message, and a `Permanent` flag; broker response text, credentials, and inner exceptions are never attached. Connection, confirmation, and broker failures are transient (the durable outbox stays the retry owner); unregistered types and invalid bindings are configuration failures.
- `RabbitMqEventingProviderStatus` — safe provider health snapshot (`rabbitmq`, health state, last stable error code).
- `AddPlatformRabbitMqEventing(IServiceCollection, Action<RabbitMqEventingOptions>?)` — validated options plus `TryAdd`ed topology, channel factory, and `IDurableEventPublisher` so application-owned registrations win.

## Platform.Caching

`Platform.Caching` is the provider-neutral cache boundary. It depends on `Platform.Core`,
`Microsoft.Extensions.Options`, and dependency-injection abstractions; it has no Redis,
HybridCache, ASP.NET Core, EF Core, or JSON dependency.

### Contracts and keys

- `ICacheStore` — asynchronous get, get-or-create, set, remove, and portable tag invalidation.
- `CacheReadResult<T>` / `CacheOperationResult` — explicit hit, miss, unavailable, and operation
  states with safe `CacheFailure` metadata.
- `CacheEntryOptions` — absolute expiration and validated portable tags.
- `CacheKey` / `CacheKeyBuilder` — bounded whitespace-free physical keys with explicit application
  and tenant prefixes.
- `ICacheProviderStatus` — safe provider health state for fail-open/fail-closed application policy.
- `CacheTelemetry` — stable activity, metric, hit, miss, and failure names that do not include raw
  keys or values.

### Implementations

- `InMemoryCacheStore` — thread-safe local store using `IClock`, absolute expiry, and tag removal.
- `Platform.Caching.Hybrid` / `HybridCacheStore` — optional Microsoft HybridCache adapter.
- `Platform.Caching.Redis` / `RedisCacheStore` — optional Redis adapter using an application-owned
  `ICacheValueSerializer`, bounded operation waits, tag sets, and unavailable-provider results.

See [`platform-caching.md`](platform-caching.md) for authority, versioning, tenant isolation,
failure policy, and migration guidance.

## Platform.Storage

`Platform.Storage` provides provider-neutral object upload, download, metadata, delete, and
presigned-operation contracts. It depends on `Platform.Core` and dependency-injection abstractions
only; cloud and filesystem dependencies remain in separate adapters.

- `StorageObjectKey` — bounded key rejecting control characters, traversal, absolute paths, and
  empty segments.
- `StorageUploadRequest`, `StorageDownloadResult`, `PresignRequest`, and `PresignedOperation` —
  explicit content, method, expiry, and size constraints.
- `IObjectStorage` — async object lifecycle boundary; `IStorageProviderStatus` exposes safe health.
- `Platform.Storage.Local` / `LocalFileStorage` — atomic local filesystem adapter.
- `Platform.Storage.S3` / `S3Storage` — optional AWS/S3-compatible adapter using an application-owned
  client and bucket configuration.

Applications own authorization, tenant prefixes, metadata tables, retention, scanning, and object
migration. See [`platform-storage.md`](platform-storage.md).

## Platform.Quota

`Platform.Quota` provides provider-neutral capacity checks and an atomic reservation lifecycle. It
depends on `Platform.Core` and dependency-injection abstractions only; it does not define plans,
prices, invoices, wallets, ledgers, or persistence models.

- `QuotaSubject`, `QuotaResource`, and `QuotaOperationKey` — bounded opaque identifiers.
- `QuotaWindow`, `QuotaDecision`, `QuotaSnapshot`, and `QuotaReservation` — explicit window and
  explanatory usage models.
- `IQuotaStore` — `CheckAsync`, idempotent `ReserveAsync`, `SettleAsync`, `ReleaseAsync`, and
  snapshot inspection.
- `InMemoryQuotaStore` — thread-safe `IClock`-driven implementation with expiration and
  `GetReservation` inspection.
- `IQuotaLimitResolver` — optional seam for application-owned entitlement-to-limit resolution.
- `Platform.Quota.Testing` — `QuotaScenarioBuilder` for deterministic test setup.
- `Platform.Quota.AspNetCore` — optional ASP.NET Core enforcement middleware, subject/resource
  resolvers, exemptions, and RFC 9457 429 responses. References `Platform.Core` and `Platform.Quota`
  and the `Microsoft.AspNetCore.App` framework reference only; it does not define plans, prices,
  entitlements, units, or persistence.

See [`platform-quota.md`](platform-quota.md) for unit conversion, reconciliation, migration, and
ownership boundaries.

## Platform.Idempotency

Framework-neutral idempotency contract. Depends on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, StackExchange.Redis, or application projects.

### Value types

- `IdempotencyRecord` — sealed record with `Key`, `Fingerprint`, `StatusCode`, `ContentType`, `ResponseHeaders` (defaulted to empty ordinal-case-insensitive map), `ResponseBody`, `CreatedAt`.

### Contracts

- `IIdempotencyStore` — `TryGetAsync(key, ct)`, `SaveAsync(record, ct)`, `EvictExpiredAsync(ct)`. `SaveAsync` rejects keys longer than the configured `MaxKeyLength` with `ArgumentException`.
- `IdempotencyMetrics` — `Hit`, `Miss`, `FingerprintMismatch`. The platform does not ship a default implementation.

### Helpers

- `RequestFingerprint.Compute(method, route, bodyHash)` — stable, hex-encoded SHA-256 of the normalised request. The method is upper-cased before hashing.
- `RequestFingerprint.ComputeBodyHash(ReadOnlySpan<byte>)` and `ComputeBodyHash(string)` — produce the matching `bodyHash` input.

### Default backend

- `InMemoryIdempotencyStore` — thread-safe default backed by a `Dictionary<string, IdempotencyRecord>` guarded by a lock. Consumes `IClock` for retention and key-staleness checks (no `DateTimeOffset.UtcNow`), honours the documented `RetentionSeconds` window, and treats expired records as misses on lookup.

### Options

- `IdempotencyOptions` — `Enabled` (default `true`), `Storage` (default `"memory"`), `RetentionSeconds` (default `86400`), `MaxKeyLength` (default `256`), `HeaderName` (default `"Idempotency-Key"`), `SectionName` constant `"Idempotency"`. Metric-name constants: `HitMetric = "idempotency.hit"`, `MissMetric = "idempotency.miss"`, `FingerprintMismatchMetric = "idempotency.fingerprint_mismatch"`.

### Registration

- `AddPlatformIdempotency(IServiceCollection)` and `AddPlatformIdempotency(IServiceCollection, Action<IdempotencyOptions>)` — bind `IdempotencyOptions` and register `IIdempotencyStore`. When `IdempotencyOptions.Enabled` is `false` the registration resolves a no-op store so consumers can opt out without changing call sites.

## Platform.RateLimiting

Framework-neutral rate-limit contract. Depends on `Platform.Core`, `Microsoft.Extensions.Options`, and `Microsoft.Extensions.DependencyInjection.Abstractions`; targets `net10.0`. Does not reference ASP.NET Core, EF Core, StackExchange.Redis, or application projects.

### Value types

- `RateLimitKey` — `readonly record struct` with `Policy` and `Subject`. `Composite` returns the documented `policy|subject` form.
- `RateLimitDecision` — sealed record with `Allowed`, `Limit`, `Remaining`, `RetryAfterSeconds`.
- `RateLimitBypassDecision` — sealed record with `Allowed` and optional `Label`.
- `IRateLimiterBackendStatus` — sealed record with `Provider`, `Available`, optional `Detail`.
- `HttpContextAbstraction` — framework-neutral wrapper with `BypassToken`, optional `Headers`, optional `RemoteIp`.

### Contracts

- `IRateLimiter` — `CheckAsync(key, cancellationToken)`. Implementations read the current time from an injected `IClock`.
- `IRateLimitBypassResolver` — `Evaluate(context)`.
- `IRateLimiterBackendStatusProvider` — `GetStatus()`. Consumer adapters can replace the registration with a provider-specific implementation.

### Default backend

- `InMemoryRateLimiter` — thread-safe default that tracks a per-key windowed counter and returns a `RateLimitDecision` with the documented fields. Reads the current time from `IClock` (no `DateTimeOffset.UtcNow`), honours the configured `WindowSeconds`, and rolls the bucket over when the window elapses. Different subjects under the same policy have independent buckets.
- `ConfigurationRateLimitBypassResolver` — default `IRateLimitBypassResolver` that matches documented bypass tokens (case-insensitive) against `HttpContextAbstraction.BypassToken`.
- `InMemoryRateLimiterBackendStatusProvider` — reports `Provider = "memory"`, `Available = true` so readiness checks can observe the backend without changing call sites.

### Policy catalog

- `RateLimitPolicies` — catalog of `RateLimitPolicyOptions { Name, Limit, WindowSeconds }`. `Find(name)` returns the matching entry or `null`. `Default()` returns the documented catalog: `feed` (60/60s), `search` (30/60s), `uploads` (5/60s), `downloads` (10/60s), `account-recovery` (3/3600s). Consumers register a custom `RateLimitPolicies` before `AddPlatformRateLimiting` to override the catalog.

### Options

- `RateLimitingOptions` — `Policies` (defaults to `RateLimitPolicies.Default()`), `BypassTokens` (defaults to empty), `SectionName` constant `"RateLimiting"`.

### Registration

- `AddPlatformRateLimiting(IServiceCollection)` and `AddPlatformRateLimiting(IServiceCollection, Action<RateLimitingOptions>)` — bind `RateLimitingOptions` and register `IRateLimiter`, `IRateLimitBypassResolver`, `IRateLimiterBackendStatusProvider`, and `IClock` when no implementation is already present.

## Platform.Domain

Framework-neutral domain primitives. Depends on `Platform.Core` only; targets `net10.0`. Does not reference ASP.NET Core, EF Core, Mediator, validation libraries, provider SDKs, or application projects.

### Contracts

- `IEntity<TId>` — typed entity identity. Identity equality and persistence remain application-owned.
- `IAggregateRoot<TId>` — combines `IEntity<TId>` with `IHasDomainEvents`. The interfaces are the preferred integration seam; applications may implement them without inheriting.
- `IDomainEvent` — transient event with `EventId`, `OccurredOnUtc`, optional `CorrelationId`/`TenantId`. Deliberately not a mediator notification; the platform never dispatches.
- `IHasDomainEvents` — `DomainEvents` (insertion order), `AddDomainEvent(@event)`, `ClearDomainEvents()`.

### Base implementations

- `Entity<TId>` — optional `IEntity<TId>` base with a protected-set `Id`.
- `AggregateRoot<TId>` — optional `IAggregateRoot<TId>` base with an insertion-ordered transient event collection. `RaiseDomainEvent` is the protected recording hook for derived behaviors.
- `DomainEvent` — optional abstract base record with `DomainEvent.Create(factory)` supplying a fresh identifier and the current UTC timestamp.

### Value types

- `Money` — sealed record with decimal `Amount` and normalized `Currency` (trimmed, uppercased invariant). `Zero(currency)` defaults to `USD`. `Add`/`Subtract` (and `+`/`-`) reject cross-currency arithmetic with a deterministic `InvalidOperationException` naming both currencies. `Multiply` (and `*`) scales by a scalar. No exchange-rate conversion; rounding policy remains application-owned.

### Markers

- `ISoftDeletable` — `IsDeleted`, `DeletedOnUtc`, `DeletedBy`. Exposed only; no query filters or delete behavior.
- `IHasTenant` — `TenantId`. Exposed only; no query filters, mappings, or isolation behavior.

### Errors

- `DomainException` — carries a stable `Platform.Core.Results.Error` with no HTTP or provider-specific status code. `Message` is always the safe `Error.Message`.
- `DomainValidationException` — carries `platform.validation`. `DomainNotFoundException` — carries `platform.not_found`. `DomainConflictException` — carries `domain.conflict` (see `DomainErrorCodes.Conflict`).

## Platform.Web

Composable web runtime foundations built on `Platform.AspNetCore`. Depends
on `Platform.AspNetCore` and the `Microsoft.AspNetCore.App` framework
reference; targets `net10.0`. Does not reference EF Core, Stripe, Mediator,
FluentValidation, or application projects. The package contributes the
runtime contracts and middleware that the optional edge packages
(`Platform.Web.Cors`, `Platform.Web.OpenApi`, `Platform.Web.Resilience`,
`Platform.Web.Versioning`) and the `Platform.Web.Telemetry` host
observability hook all share.

- `PlatformWebOptions` — bounded options used by every `Platform.Web.*`
  package and by `Platform.Starter` (route prefix defaults, environment
  names, validation entry points).
- `RuntimeContracts` — request, response, and tag shapes the runtime
  middleware and the edge packages read from (without prescribing a
  specific ASP.NET Core filter or handler type).
- `RuntimeMiddleware` — minimal `UsePlatformWebRuntime(IApplicationBuilder)`
  helper that wires the documented order (telemetry, correlation, error
  mapping, readiness).
- `PlatformWebAssemblyMarker` — `partial class Program` marker so
  integration tests can host the assembly in-memory (the same pattern
  the `Platform.Testing.AspNetCore` factory relies on).

`Platform.Web` is the dependency root for `Platform.Starter`. Consumers
adopting only one of the edge packages do not need to take `Platform.Web`
explicitly — the edge package brings the foundation transitively.

## Platform.Web.Composition

Optional explicit ASP.NET Core module composition. Depends on `Platform.Core` plus the `Microsoft.AspNetCore.App` framework reference; targets `net10.0`. Does not reference Mediator, FluentValidation, EF Core, provider SDKs, or application projects.

### Contracts

- `IPlatformWebModule` — stable `Name`, deterministic `Order`, required `ConfigureServices(IServiceCollection)`, and default no-op `ConfigureMiddleware(IApplicationBuilder)` / `MapEndpoints(IEndpointRouteBuilder)` hooks. The interface is the integration seam; modules need only a public parameterless constructor.
- `PlatformWebModuleRegistry` — immutable ordered snapshot (`Order`, then `Name` ordinal) built per host via `FromProvider(provider)`. Registry state lives in DI, so parallel hosts keep independent module sets.

### Registration

- `AddPlatformWebModule<TModule>()` and `AddPlatformWebModule(Type)` — explicit registration only. Each call instantiates the module, validates its name, invokes `ConfigureServices`, and records the instance. Duplicate types or names fail deterministically with `InvalidOperationException`. Nothing is scanned: unregistered module types are never instantiated or mapped.

### Pipeline

- `UsePlatformWebModules(app)` — invokes each `ConfigureMiddleware` hook at most once, in registry order. Opt-in: without the call, registered modules contribute no middleware.
- `MapPlatformWebModules(endpoints)` — invokes each `MapEndpoints` hook at most once, in registry order. Opt-in: without the call, registered modules map no endpoints.

## Platform.Persistence.EfCore

Optional provider-neutral EF Core conventions. Depends on `Platform.Core`, EF Core, relational
abstractions, and health-check abstractions; targets `net10.0`. It does not own application
entities, contexts, migrations, tenants, or business filters.

- `IAuditableEntity`, `ISoftDeletable`, `ITenantScoped`, and `ITenantScope` — minimal contracts
  for explicitly selected application entities and scopes.
- `PlatformSaveChangesInterceptor` — opt-in audit and soft-delete interception using replaceable
  `IClock` and `IActorAccessor` services.
- `PageRequest`, `PageResult<T>`, `PageExtensions`, `Specification<T>`, and
  `SpecificationEvaluator` — bounded paging and predicate composition over consumer queryables.
- `EfCoreMigrationStatusReader`, `MigrationStatus`, and `EfCoreReadinessCheck` — read-only
  connection/migration status; no implicit migration application.
- `AddPlatformPersistenceEfCore` and `ModelBuilderExtensions` — explicit registration and
  per-entity soft-delete/tenant filter helpers.

## Platform.Persistence.EfCore.Migrator

Optional application-owned EF Core migration execution boundary. Depends on
`Platform.Persistence.EfCore` plus provider-neutral EF Core (`Microsoft.EntityFrameworkCore`,
`Microsoft.EntityFrameworkCore.Relational`); targets `net10.0`. It owns no contexts, migrations,
connection strings, locks, tenant iteration, or seed data, and references no provider, web,
messaging, or job packages.

- `IMigrationRunner` / `MigrationRunner` — `ListPendingAsync` returns pending identifiers
  without modifying the database (a missing database reports every defined migration as
  pending); `ApplyAsync` captures pending, calls `MigrateAsync` when non-empty, then runs the
  seed callback when requested. Cancellation propagates `OperationCanceledException`.
- `MigrationRunnerRequest` — application-owned factory plus `SeedAfterApply`, optional
  `IMigrationSeeder`, and optional `IMigrationExclusiveExecutor` (absent means no lock).
- `MigrationPendingResult`, `MigrationRunResult`, `MigrationFailure` — stable
  `MigrationFailureCategory` (`Unavailable`, `MigrationFailed`, `SeedFailed`) with fixed
  secret-free diagnostics (category plus exception type only; never connection strings, SQL,
  exception messages, or provider bodies).
- `MigrationCommand` / `MigrationConsoleRunner` — thin console-host adapter owning argument
  parsing (`apply` default, `list-pending`, `--seed`, `-h|--help`) and exit codes
  (0 success/help, 1 failure, 2 usage error).

## Platform.Adoption

Provider-neutral adoption diagnostics core. Depends on nothing but the
`net10.0` framework; targets `net10.0`. It inspects an explicit absolute
target directory and never modifies it.

- `AdoptionAnalyzer` — deterministic checks: SDK pin (`global.json`),
  solution/project discoverability, central package management,
  exact `Platform.*` pinning (floating versions fail, unpinned or
  outdated pins warn), production-to-`*.Testing` boundary, and
  `Nullable`/`TreatWarningsAsErrors` signals, plus opt-in
  environment-blocked feed/Docker probes with rerun instructions.
- `AdoptionReport` / `AdoptionCheckResult` / `AdoptionStatus`
  (`Pass`, `Warning`, `Failed`, `EnvironmentBlocked`) — stable
  machine-readable JSON with secret-free evidence (relative paths,
  identifiers, versions, exception type names only).
- `AdoptionOptions`, `AdoptionExitCodes` (0 clean, 1 failures,
  2 blocked-only, 64 usage), `ProposedEdit` preview-only alignment
  suggestions, `AdoptionInventory` project/package listing,
  `AdoptionPreview` JSON, and `AdoptionTargetException` for unusable
  targets.
- `AdoptionEvidenceLevel` (`Absent`, `Configured`, `Incompatible`,
  `Unverified`, `Verified`), `AdoptionEvidence`
  (`IsProductionReady` only for `Verified`), and the deterministic,
  I/O-free `AdoptionEvidenceClassifier` — a referenced package without
  native consumer evidence classifies as `Configured` or `Unverified`,
  never production-ready.

The `platform-doctor` CLI (`tools/Platform.Adoption.Tool`, unpacked)
adds the `doctor`, `inventory`, `conformance`, and `preview` commands
over this core with the same exit codes.

## Platform.Persistence.Postgres

Optional Npgsql adapter over `Platform.Persistence.EfCore`. It contains only PostgreSQL options
configuration (`UsePlatformPostgres`) and does not add contexts, migrations, or domain behavior.

## Platform.Persistence.Multitenancy

Optional multitenancy adapter over `Platform.Persistence.EfCore` and `Platform.AspNetCore`.
Targets `net10.0`. Depends on `Platform.Core`, `Platform.AspNetCore`, `Platform.Persistence.EfCore`,
EF Core, and health-check abstractions. The package owns no tenant entities, tenant catalog,
migrations, or database credentials.

### Tenancy contracts (`Platform.Core`)

- `ITenantInfo` — application-supplied tenant metadata.
- `TenantResolutionResult`, `TenantResolutionStatus` — resolution outcome (resolved, global operation, or unresolved).
- `ITenantResolver` — application-owned resolver; the platform never inspects HTTP headers or claims directly.
- `IAmbientTenantScope`, `AmbientTenantScope` — current ambient state; the package adds a generation counter so cached collaborators can detect replacement.
- `ITenantScopeFactory` — installs and restores ambient scopes for background handlers and explicit global operations.
- `ITenantConnectionResolver`, `TenantConnectionDescriptor` — connection routing seam for shared and dedicated tenant databases.
- `ITenantConnectionReadinessProbe`, `TenantConnectionReadinessResult` — readiness probe contract.
- `TenantScopeNotResolvedException` — fail-closed outcome when no scope is in effect.

### Adapter

- `AddPlatformPersistenceMultitenancy` and `MultitenancyOptions` — bounded registration with safe defaults.
- `AmbientTenantScopeStore`, `ITenantScopeAccessor` — scoped accessor over an `AsyncLocal<AmbientTenantScope>`.
- `TenantScopeFactory` — default `ITenantScopeFactory` with snapshotted prior-scope restoration.
- `TenantScopeMiddleware` and `UsePlatformMultitenancy` — ASP.NET Core middleware that resolves and installs the ambient scope.
- `IGlobalTenantEntity` — explicit global marker that opts an entity out of the default filter.
- `TenantModelBuilderExtensions.ApplyDefaultTenantFilters(scope[, behavior, overrideTenantId])` — explicit per-`DbContext` model customizer.
- `ScopedTenantConnectionProvider` — returns the connection descriptor for the current scope; caches within a single generation.
- `TenantConnectionReadinessCheck` and `AddPlatformTenantConnectionReadinessCheck` — readiness aggregation for tenant connections.

## Platform.Tenant.Lifecycle (contracts)

Provider-neutral tenant provisioning and lifecycle orchestration. Framework-neutral; targets `net10.0`. Zero third-party packages and no project references.

### State machine

- `TenantLifecycleStepOutcome` — `Succeeded`, `Retryable`, `Permanent`, `Canceled`, `PolicyDenied`.
- `TenantLifecycleOperationState` — `Pending`, `Running`, `Succeeded`, `Retryable`, `PermanentlyFailed`, `Canceled`, `PolicyDenied`. The store refuses to transition a terminal state.
- `TenantLifecycleOperationId` / `TenantLifecycleStepName` / `TenantLifecycleWorkflowName` — opaque, stable identifiers.
- `TenantLifecycleStepContext` — operation, workflow, step, tenant id, attempt counter, and an opaque metadata bag the application populates.
- `TenantLifecycleStepResult` / `TenantLifecycleStepStatus` / `TenantLifecycleOperationStatus` — safe, secret-free status snapshots.
- `TenantLifecycleReasons` — provider-neutral reason constants for readiness (`ready`, `running`, `retryable`, `permanently_failed`, `canceled`, `policy_denied`, `unknown`).

### Workflow

- `ITenantLifecycleStep` — application-owned step. `Name` is the idempotency boundary; `Order` resolves execution order; `IsTenantScoped` tells the orchestrator to install the scope around the step.
- `ITenantLifecycleWorkflow` — ordered, named list of steps; the workflow name plus the operation id is the durable identity for resume.
- `ITenantLifecycleScopeCallback` — application hook that installs and restores the tenant scope. The platform never imports the multitenancy adapter directly.
- `ITenantLifecycleStore` — application-owned durable store; the platform provides only the contract and an in-memory test implementation.
- `ITenantLifecycleOrchestrator` — `StartAsync` and `ResumeAsync`; the orchestrator skips already-completed steps and classifies step outcomes into operation states.

## Platform.Tenant.Lifecycle

Default orchestrator. Depends on `Platform.Tenant.Lifecycle.Contracts` and `Platform.Core` (for `IClock` and logging abstractions). No EF Core, Hangfire, Quartz, or ASP.NET Core references.

- `TenantLifecycleOrchestrator` — `ITenantLifecycleOrchestrator` default. Runs ordered steps, installs and disposes the tenant scope around each `IsTenantScoped` step, and routes step outcomes to the operation state.
- `TenantLifecycleWorkflowRegistry` / `ITenantLifecycleWorkflowRegistry` — workflow-by-name lookup the orchestrator uses to resume an operation.
- `InMemoryTenantLifecycleStore` — in-memory, non-production store. Production callers replace it with a durable adapter.
- `TenantLifecycleOrchestratorExtensions.WithWorkflowRegistry` — fluent attachment for the registry; required for `ResumeAsync` to locate the workflow by name.
- `AddPlatformTenantLifecycle(IServiceCollection)` / `AddPlatformTenantLifecycle(IServiceCollection, Action<TenantLifecycleOptions>)` — DI registration. Replaces the in-memory store with the application-owned adapter in production.

## Platform.Tenant.Lifecycle.AspNetCore

Status and readiness adapter. Depends on `Platform.Tenant.Lifecycle`, `Platform.Tenant.Lifecycle.Contracts`, and the `Microsoft.AspNetCore.App` framework reference. No third-party packages.

- `AddPlatformTenantLifecycleReadiness` — registers the platform readiness check that maps the most recent operation state to the readiness taxonomy.
- `TenantLifecycleReadinessCheck` — default readiness check; reads the store, maps `Succeeded` / `Running` / `Pending` to healthy and every terminal failure to unhealthy with a stable reason.
- `MapPlatformTenantLifecycleStatus` / `MapPlatformTenantLifecycleResume` — minimal-API mappers for the status and operator-driven resume flows.
- `IReadinessCheck` / `ReadinessResult` / `ReadinessContext` — provider-neutral readiness surface that the host's readiness endpoint adapts.

## Platform.Tenant.Lifecycle.Testing

Deterministic fakes. Depends on `Platform.Tenant.Lifecycle.Contracts`. No third-party packages.

- `InMemoryTenantLifecycleStore` — test in-memory store with linearizable status reads and writes.
- `ScriptedLifecycleStep` — step that consumes pre-configured results; otherwise returns the configured default. `Invocations` records every call.
- `DelegateLifecycleStep` — step that delegates to a caller-supplied `Func<TenantLifecycleStepContext, CancellationToken, ValueTask<TenantLifecycleStepResult>>`.
- `StaticLifecycleWorkflow` — composes a workflow from a list of steps in declaration order.
- `RecordingLifecycleScopeCallback` — records every `BeginTenantScope` invocation and tracks the active scope count.

## Platform.Testing

Test-only helpers. Depends on `Platform.Core`, `Platform.AspNetCore`, and `Platform.Billing.Contracts`. No xUnit, NUnit, or mocking-framework dependencies. Targets `net10.0`. Production projects must not reference this package.

### Time

- `ControllableClock` — deterministic `IClock` that never reads system time. `Set(DateTimeOffset)`, `Advance(TimeSpan)`. Rejects non-UTC values. Initial value defaults to `DateTimeOffset.UnixEpoch`.

### Entitlements

- `SubscriptionBuilder` — fluent builder for `Subscription`. Defaults to active status, `plan.test`, `test` provider, `sub_test` provider id, and a 30-day period anchored at the supplied `IClock`. `WithPlan`, `WithStatus`, `WithProvider`, `WithProviderSubscriptionId`, `WithPeriodStart`, `WithPeriodEnd`.
- `EntitlementBuilder` — fluent builder for `Entitlement`. Safe inactive defaults (no features, no limits). `WithTenant`, `WithSubscription`, `Granting(feature)`, `Granting(IEnumerable<FeatureKey>)`, `WithLimit(feature, limit)`, `CapturedAt`.
- `FakeEntitlementStore` — in-memory store with `Configure`, `Get`, `Invalidate`, `InvalidatedSubjects`, `Reset`. `Get` returns `EntitlementDefaults.Inactive` for unknown or invalidated subjects.

### Usage

- `UsageCall` — record describing one recorded call: `Subject`, `Feature`, `Units`, `OperationKind` (`Check` or `Record`).
- `RecordingUsageMeter` — `IUsageMeter` that records every call and accumulates totals per `(subject, feature)`. `SetLimit(feature, limit?)` configures per-feature limits; `TotalFor`; `Calls`; `Reset` clears totals and calls but preserves limits.

### Eventing

- `RecordingEventBus` — `IEventBus` that records every published `IntegrationEventEnvelope` in invocation order, supports `EnvelopesOfType(payloadType)` filtering, and `Reset` clears the recorded envelopes. The fake never throws; tests that need failure behaviour pair it with the failure-injection helper.

### Failure injection

- `InjectedFailure` — record describing a single emission: `Label` (public, exception-message-free identifier), `Kind` (`Transient` or `Permanent`), `Sequence` (order in which the injector emitted it).
- `InjectedFailureKind` — `Transient` failures exhaust on the next call; `Permanent` failures persist until `Reset`.
- `TransientFailureInjector` — fluent configuration with `WithTransient(label, exception)` and `WithPermanent(label, exception)`. `Run(label, action)` / `RunAsync(label, action)` inject the configured exception on the next call. `History` returns a snapshot of every emission; `Reset` clears pending injections and history.

## Platform.Testing.AspNetCore

ASP.NET Core TestServer host builder. Depends on `Platform.Core` and the `Microsoft.AspNetCore.App` framework reference. The package pulls `Microsoft.AspNetCore.Mvc.Testing` transitively. Production projects must not reference this package.

- `PlatformTestWebApplicationFactory<TEntryPoint>` — a real `WebApplicationFactory<TEntryPoint>` that boots the **consumer's own** application over `TestServer` from the named entry point. It never builds an application of its own, and it never falls back to one: an entry point it cannot boot raises a named diagnostic naming the type, its declaring assembly, and the `InternalsVisibleTo` / public-marker fix. The type argument is the application's `Program` or a public marker type (C# has no default type parameter, so it is always required). Inherits `ConfigureWebHost`, `WithWebHostBuilder`, `CreateClient()`, `Services` and `Server`; every configuration callback runs against the consumer's host **after** the application's own service registrations, so an application-registered service can be replaced. **One host per instance**: every member below mutates this factory, so repeated access reuses the single host, and `WithWebHostBuilder` — which derives a second factory and builds a second host — is not the way to configure a scenario. Tests should `await using` it.
  - `WithEnvironment(string)` — selects the environment; any non-empty name is honoured and the consumer's `appsettings.<Environment>.json` applies. Defaults to `PlatformTestEnvironments.Testing`. Throws `ArgumentException` on null/empty/whitespace.
  - `WithConfiguration(key, value)` — in-memory value that outranks the consumer's own configuration sources.
  - `ConfigureTestServices((services, configuration) => ...)` and `ConfigureWebHost(builder => ...)` — fluent host-configuration seams, so a scenario needs no derived class.
  - `ReplaceService<TService>(instance)`, `ReplaceService<TService, TImplementation>()` — remove **every** prior registration and register one. Throws a named `InvalidOperationException` if the application never registered the service.
  - `DecorateService<TService, TWrapper>(decorate)` — the original instance is resolved and handed to `decorate`, so it stays reachable inside the wrapper.
  - `WithClock(IClock)` and `WithClock(TimeProvider)` — deterministic clock injection bound to `Platform.Core.Time.IClock` and to the BCL `TimeProvider`; no new package reference.
  - `OnDispose(Action)` / `OnDispose(Func<ValueTask>)` — provider-agnostic cleanup. Actions run only after the host has stopped, in reverse registration order; a failing action neither stops the others nor host disposal, and is surfaced to the caller of `DisposeAsync`.
  - `UseTemporaryDirectory(scope)` / `TemporaryDirectory` — a unique writable directory under the OS temp root, removed on disposal, never a content root and never the process working directory. `PerFactory` (default) is parallel-safe; `ProcessWide` is an explicit, reference-counted opt-in.
  - Every mutating member throws a named `InvalidOperationException` once the host exists, pointing at "configure before first access" rather than at a second host build.
- **Not provided, by design:** no `ReplaceDbContext`, `MigrateOnStartup`, `EnsureCreatedAsync` or `ResetDatabaseAsync` (they would need EF Core in a package four repositories consume); no `IAsyncLifetime` (xUnit in a shipped package); no `CaptureLogs`, `WithDefaultRequestHeader`, fake-auth or startup-failure helpers (`ConfigureWebHost` and the inherited `CreateClient`/`ConfigureClient` cover those shapes). Database isolation and schema bootstrap stay with the consumer through the configuration overlay, the host-configuration seam and `OnDispose`.
- `PlatformTestHostConfiguration` — delegate signature passed to `ConfigureTestServices`; receives the service collection and an `IConfigurationBuilder` so callers can swap services for fakes or extend the configuration surface.
- `PlatformTestEnvironments.Testing` — the canonical environment name used by the factory.

## Test projects

| Project | Coverage |
| --- | --- |
| `tests/Platform.Architecture.Tests` | Dependency-direction guardrails (no production project references test projects, ASP.NET Core, EF Core, or Stripe; `FrameworkReference` is allowed only for `Platform.AspNetCore`; `Platform.Core` and `Platform.Billing.Contracts` declare no `<PackageReference>` entries; per-package forbidden-reference rules for `Platform.Jobs`, `Platform.Mailing`, `Platform.Eventing`, `Platform.Idempotency`, `Platform.RateLimiting`, `Platform.Domain`, `Platform.Web.Composition`, `Platform.Persistence.EfCore.Migrator`, and `Platform.Adoption`; the template pack stays content-only; every `samples/` project stays unpacked). |
| `tests/Platform.Core.Tests` | Unit tests for time, results, context, and audit contracts. |
| `tests/Platform.AspNetCore.Tests` | Unit tests for the ProblemDetails mapper, exception middleware, and correlation middleware, plus a `TestServer` integration test for the minimal host (known failure → ProblemDetails, unknown failure → sanitized 500, correlation generation, health endpoint). |
| `tests/Platform.Billing.Contracts.Tests` | Unit tests for identifiers, subscription status and period boundaries, entitlement defaults, feature check decisions, usage-meter contract (in-memory implementation), and processed-event idempotency. |
| `tests/Platform.Jobs.Tests` | Unit tests for the `RecurringJobAttribute` reflection, the `RecurringJobDescriptor` value type, the `IJobTelemetry` surface, and `BackgroundJobsOptions` defaults, plus a `TestServer` integration test for `AddPlatformJobs` defaults and configuration overrides. |
| `tests/Platform.Jobs.Hangfire.Tests` | Options defaults and validation, dispatcher payload serialization and telemetry, recurring registration (first-wins, time zone, failure rollback), executor payload normalization and redacted failure telemetry, context capture/restoration filters and activator (including fail-closed without a bridge), dashboard factory and TestServer authorization coverage, storage health with redacted diagnostics, idempotent/consumer-override DI registration, end-to-end host execution (dispatched and recurring jobs with context restoration), and a Docker-gated PostgreSQL storage integration test. |
| `tests/Platform.Mailing.Tests` | Unit tests for `MailAddress`, `MailAttachment`, `MailMessage` validation (text-only / html-only / both / neither / empty recipients / null sender / empty subject), `MailSendResult` semantics, `MailTemplateId` implicit conversions, `RenderedMailTemplate.Create` validation, and `MailingOptions` defaults, plus a `TestServer` integration test for `AddPlatformMailing` defaults, configuration overrides, and a consumer-registered `IMailService`. |
| `tests/Platform.Mailing.ProviderAdapters.Tests` | Deterministic adapter coverage for `Platform.Mailing.Smtp` (in-process fake SMTP server: MIME payload, attachments, authentication, recipient/mail rejection classification, connection refusal, operation timeout, cancellation preservation, configuration failures without server contact, options validation) and `Platform.Mailing.SendGrid` (fake `ISendGridClient`: response classification, message mapping, transport failure, cancellation, configuration failures, options validation), plus opt-in DI registration and consumer-override semantics for both adapters. |
| `tests/Platform.Eventing.RabbitMq.Tests` | Options defaults and validation, topology mapping and fail-closed defaults, confirmed publishing with deterministic routing, unregistered/unconfigured/invalid-binding configuration failures, connect and confirm timeouts, cancellation preservation, broker-failure reconnection, exchange-declaration opt-in, DI registration and consumer-override semantics, an unreachable-broker transient classification, and a Docker-gated RabbitMQ broker integration test. |
| `tests/Platform.Eventing.Tests` | Unit tests for the envelope shape, the default `IntegrationEventEnvelopeSerializer` and `IntegrationEventEnvelopeDeserializer`, the `InProcessEventBus` (typed dispatch, consumer-failure isolation, idempotent disposal, bounded-capacity null guard), and `EventingOptions` defaults, plus a `TestServer` integration test for `AddPlatformEventing` and `AddPlatformEventingInProcess` defaults, configuration overrides, and a consumer-published envelope flowing to a typed `IIntegrationEventHandler<>`. |
| `tests/Platform.Idempotency.Tests` | Unit tests for `RequestFingerprint` stability + method normalisation + body-hash helper, `IdempotencyOptions` defaults and metric-name constants, `InMemoryIdempotencyStore` round-trip / null-or-empty-key / oversize-key / retention sweep / expired-record-as-miss / null-dependency guards, plus a `TestServer` integration test for `AddPlatformIdempotency` defaults, configuration overrides, save/try-get round-trip, eviction sweep driven by a `MutableClock`, and the no-op path when `Idempotency:Enabled = false`. |
| `tests/Platform.RateLimiting.Tests` | Unit tests for `RateLimitPolicies` default catalog + `Find` + invalid-entry dropping + null guard, `RateLimitingOptions` defaults, `InMemoryRateLimiter` (first request, burst over limit, window roll-over, per-subject isolation, unknown/empty policy / subject rejection, null dependency guards), `ConfigurationRateLimitBypassResolver`, and `InMemoryRateLimiterBackendStatusProvider`, plus a `TestServer` integration test for `AddPlatformRateLimiting` defaults, configuration overrides, limiter decisions through DI, and the readiness surface. |
| `tests/Platform.Domain.Tests` | Unit tests for typed entity identity (base class and contract-only), aggregate event recording in insertion order + clearing without dispatch + null guard + contract-only aggregate, `DomainEvent.Create` identifier/timestamp semantics, `Money` currency normalization/validation/arithmetic/cross-currency rejection/null guards, opt-in soft-delete and tenant markers, and safe domain errors (stable `Error` carriage, validation/not-found/conflict codes, web-boundary mapping without inspecting internals). |
| `tests/Platform.Web.Composition.Tests` | Unit tests for explicit registration (service hook invocation, `Order`-then-`Name` ordering, duplicate type/name rejection, empty-name and missing-constructor rejection, non-module and null-argument guards) and the ordered per-host registry snapshot, plus `TestServer` integration tests for middleware running in module order, endpoints mapped once per call, no pipeline contribution without the opt-in extensions, unregistered module types never instantiated or mapped, and two hosts in one process keeping independent module sets. |
| `tests/Platform.Persistence.EfCore.Tests` | In-memory and SQLite tests for explicit options, audit/soft-delete interception, tenant filters, paging/specification helpers, concurrent independent contexts, read-only migration status, and readiness behavior. |
| `tests/Platform.Persistence.EfCore.Migrator.Tests` | Unit tests for request validation, cancellation propagation, exclusive-executor delegation and result pass-through, seed suppression on failure, factory failure redaction, and result-factory guards (InMemory contexts, recording callbacks), plus SQLite integration tests with an application-owned context and hand-written migration covering read-only pending inspection, apply-to-head, seed-inside-lock ordering with seed-on-no-op, seed-failure classification without leaking the cause, and unavailable classification without leaking the path, plus console-adapter tests for parsing, exit codes, seed-flag enablement, configuration-failure redaction, and cancellation. |
| `tests/Platform.Adoption.Tests` | Analyzer unit tests over minimal/adopted/misconfigured fixtures (target validation, per-check statuses, preview proposals, explicit sibling-directory targeting, unreachable-feed blocked classification with rerun guidance, environment opt-in, stable secret-free JSON, inventory listing) with hash-snapshot read-only proof, plus CLI tests for doctor/conformance/preview output, exit codes (0/1/64), and preview write-freedom; the `AdoptionEvidenceClassifier` truth table covers absent, incompatible, configured, unverified, and verified levels. | |
| `tests/Platform.SampleMatrix.Tests` | Per-stage wiring tests (minimal-web root plus `/live`, EF Core migration apply plus insert/query round-trip, identity login success/401/400, ordered two-step tenant provisioning with durable status, local storage upload/download/metadata/delete) plus a metadata test asserting every `samples/matrix.json` entry matches its project (references, framework, unpacked marker, README, rollback) and no sample references another sample. |
| `tests/Platform.Persistence.Multitenancy.Tests` | Multitenancy options validation, scope factory installation/restoration, EF Core model filter application, global-entity opt-out, scoped connection routing (tenant/global/shared), connection caching, HTTP middleware TestServer coverage (resolved/disabled/length-bounded), and tenant readiness check aggregation. |
| `tests/Platform.Tenant.Lifecycle.Tests` | Tenant lifecycle orchestrator: ordered execution + succeeded status, retryable classification stops the run, permanent classification fails closed, cancellation transitions to `Canceled`, tenant scope is installed and disposed around every tenant-scoped step, duplicate step names are rejected, `ResumeAsync` skips completed steps and recovers, `ResumeAsync` throws on unknown operations and un-registered workflows, and safe messages are preserved on step status records; status endpoint (404 for unknown operations, OK with snapshot), resume endpoint (operator-driven run to completion), readiness check (healthy for succeeded, unhealthy for retryable). |
| `tests/Platform.Persistence.Postgres.Tests` | Provider-boundary test for PostgreSQL options configuration. |
| `tests/Platform.Testing.Tests` | Unit tests for `ControllableClock`, `SubscriptionBuilder`, `EntitlementBuilder`, `FakeEntitlementStore`, and `RecordingUsageMeter`; toolkit tests for the `RecordingEventBus` (ordering, payload-type filter, reset, cancellation, null guard), `TransientFailureInjector` (transient and permanent, history ordering, reset, unknown label pass-through, async propagation); `PlatformTestWebApplicationFactory<TEntryPoint>` against the real in-repo test application `tests/Platform.Testing.TestApplication` (boots the consumer's application and reaches its own routes with no factory-added route; testing environment default and `WithEnvironment`; in-memory configuration outranking `appsettings.json`; static assets served from the application's own output directory; `ConfigureTestServices`/`ConfigureWebHost` service and configuration seams; replace/decorate including the unregistered-service and null-argument failures; `IClock` and `TimeProvider` injection; teardown ordering, awaiting, failure isolation and surfacing; per-factory and process-wide temporary directories; one-host-per-instance, configuration permutations without subclassing, cross-factory isolation and named entry-point diagnostics); and an end-to-end checkout scenario that wires the toolkit into a real host. |
| `tests/Platform.Webhooks.Tests` | Synthetic signature, replay, normalization, and HTTP mapping coverage for the inbound and outbound flows. |
| `tests/Platform.Auditing.Tests` | Event validation, default masking rules, enricher/recorder semantics, `InMemoryAuditSink` and options, HTTP middleware capture (request, exception, security status, fail-open), exception classification, and EF Core `IAuditedEntity` change capture with masking and diffs. |
| `tests/Platform.Web.Edge.Tests` | Telemetry contract, CORS option and TestServer coverage, HTTP resilience option/handler/circuit-breaker coverage, OpenAPI registry and TestServer coverage. |
| `tests/Platform.Identity.Tests` | Identity contract coverage (anonymous user, fake credential verifier, fake external provider, permission catalog), JWT options validation (disabled-by-default, required fields, redacted diagnostics), ASP.NET Core identity host integration (claim projection, default/empty claim, anonymous fallback, permission handler, audit hook), and the new identity lifecycle contracts (refresh-token rotation + replay + expiry + revocation + 32-thread concurrent rotation + audit events; password recovery with no-enumeration for known/unknown subjects, replay rejection, invalid-challenge handling; two-factor challenge/verify with wrong code, unknown challenge, empty subject; impersonation fail-closed default, allow policy grants, active context lookup, end-after-start, unknown-grant end, invalid request shape; endpoint integration via `TestServer` for refresh rotation, refresh replay, password-recovery initiation returning `202` for known and unknown subjects, two-factor challenge + verify, and impersonation start failing closed without a policy). |
| `tests/Platform.Web.Versioning.Tests` | `PlatformWebVersioningOptions` validation (default values, negative `DefaultMajor`, out-of-range `DefaultMinor`, reader-specific names, format/constraint non-emptiness); reader selection (URL segment default, header, query, media type, composite) and explorer options propagation (`GroupNameFormat`, `ReportApiVersions`, `RouteConstraintName`); `TestServer` coverage for opt-in behavior, default-version assumption, URL/header/query readers, two-version API Explorer groups, and the `IPlatformVersioningDefaultsProvider` seam. |
| `tests/Platform.ConsumerConformance` | Test-only consumer fixture that restores platform packages from a local NuGet feed and verifies registration, replacement, health, failure classification, opt-in boundaries, end-to-end host behavior, and the canonical shared-contract envelopes (`identity-subject`, `permission`, `tenant`, `audit`, `gate-result`, `release-evidence`). Driven by `scripts/conformance.sh`; intentionally not part of `Platform.sln`. The fixture also enforces adoption conformance: every `Platform.*` reference is pinned to the same exact version, every `<X>.Testing` package is paired with the matching `<X>.Contracts` partner, the local `eng/package-manifest.json` is in sync with the source, and the upgrade/rollback smoke script exists. |
## Platform.Admin.Contracts

Provider-neutral administration contracts. Framework-neutral; targets
`net10.0`. Zero third-party dependencies. The package defines the
admin records (`IAdminStore`, `AdminRecord`, `AdminRole`,
`AdminPermission`) and the application-owned extension points; it does
not own the persistence, the auth scheme, the routes, or the
endpoints.

## Platform.Admin.AspNetCore

Opt-in ASP.NET Core administration endpoints. Depends on
`Platform.Admin.Contracts`, `Platform.Identity.Contracts`, and
`Platform.Identity.AspNetCore` plus the `Microsoft.AspNetCore.App`
framework reference; targets `net10.0`. Does not reference EF Core,
Stripe, or application projects.

- `AddPlatformAdmin(IServiceCollection)` — registers the admin
  endpoints, the audit hook, and the request/response shapes. The
  application supplies the `IAdminStore` (in-memory test default or
  application-owned durable adapter) and the authorization policy.
- `MapPlatformAdminEndpoints(IEndpointRouteBuilder)` — maps the
  documented routes (`/admin/users`, `/admin/roles`,
  `/admin/permissions`, `/admin/audit`) with the platform's safe
  `ProblemDetails` mapping and the platform correlation identifier.
- `PlatformAdminOptions` — bounded options: route prefix, audit
  category name, page size, retention window, JSON content type.
  `Validate()` rejects empty prefixes and non-positive page sizes.

## Platform.Admin.Testing

Deterministic admin test doubles. Depends on `Platform.Admin.Contracts`;
targets `net10.0`. Production projects must not reference this package.

- `InMemoryAdminStore` — thread-safe, in-memory `IAdminStore` with
  `Add`, `Get`, `Update`, `Remove`, `List`, `Reset`, and
  `Invocations` for assertions. Used by `tests/Platform.Admin.Tests`
  and the `Platform.Starter.Sample` end-to-end sample.

## Platform.Identity (contracts)

Provider-neutral identity and authentication contracts. Framework-neutral; targets `net10.0`. Zero third-party dependencies.

### Current user

- `CurrentUser` — immutable subject/email/tenant/roles/permissions projection. `IsAuthenticated`, `RoleSet`, `PermissionSet`, and `Anonymous`.
- `ICurrentUserAccessor` — resolves the current user without prescribing a token or user entity.

### Providers

- `ICredentialVerifier`, `IExternalIdentityProvider`, `IVerificationProvider`, `ISessionStore` — replaceable contracts returning `IdentityProviderResult<T>` with a normalized `IdentityFailureReason`. Secrets, provider response bodies, and vendor exceptions are never placed in the result.
- `IIdentityAuditHook` — receives `IdentityAuditEvent` for security-sensitive mutations without prescribing a store.

### Lifecycle

- `IdentityLifecycleOutcome` / `IdentityLifecycleResults` / `IdentityLifecycleResult<T>` — stable, provider-neutral outcome codes (`Succeeded`, `InvalidHandle`, `Expired`, `Revoked`, `Replayed`, `PolicyDenied`, `PreconditionNotMet`, `ProviderUnavailable`, `InvalidRequest`, `Unknown`).
- `IRefreshTokenStore` / `IRefreshTokenService` — atomic consume-and-replace rotation. `RefreshToken`, `RefreshTokenRotation`, `DefaultRefreshTokenService`, `IIdentityLifecycleCoordinator` compose application-owned stores with the platform audit hook.
- `IPasswordRecoveryService` — initiate and complete; safe-failure semantics return the same `Succeeded` outcome for unknown and known subjects.
- `ITwoFactorService` — opaque `TwoFactorChallenge` with channel selection; verification reports `PolicyDenied` for mismatched codes and `Expired` past the issued window.
- `IImpersonationPolicy` / `IImpersonationService` / `ImpersonationGrant` / `ImpersonationContext` / `ImpersonationAuthorizationRequest` — fail-closed impersonation: without a registered policy, every `StartAsync` call is denied and the platform records an `identity.impersonation.denied` audit event.
- `PasswordRecoveryChallenge` / `TwoFactorChallenge` — opaque handles only; the platform never sees the user's enrolled factors or the raw codes.

## Platform.Authorization

Module-owned permission definitions and authorization decision contracts. Framework-neutral; targets `net10.0`.

- `PermissionDefinition` / `PermissionCatalog` — resource/action permission keyed by `resource.action`; the catalog is module-owned and rejects duplicate keys.
- `PlatformPolicyNames` — stable `ForPermission` / `ForRole` policy names.
- `AuthorizationDecision` / `IAuthorizationDecisionAuditor` — normalized decision and recording sink.

## Platform.Identity.AspNetCore

Optional ASP.NET Core host adapter. Depends on `Platform.Identity.Contracts` and `Platform.Authorization` and the `Microsoft.AspNetCore.App` framework reference; no third-party packages.

### Registration

- `AddPlatformIdentity()` / `AddPlatformIdentity(Action<PlatformIdentityOptions>)` — register `HttpCurrentUserAccessor`, `IIdentitySessionService`, the permission handler, and authorization.
- `AddPlatformIdentityAuthentication()` — registers the platform authentication scheme without replacing consumer schemes.
- `AddPlatformIdentityCredentialVerifier<T>` / `AddPlatformIdentityExternalProvider<T>` / `AddPlatformIdentityVerificationProvider<T>` / `AddPlatformIdentitySessionStore<T>` / `AddPlatformIdentityAuditHook<T>` — `TryAdd` seams for application-owned providers and stores.
- `RequirePlatformPermission("resource.action")` / `RequirePlatformRole("role")` — named policies; `PlatformPermissionPolicy` / `PlatformRolePolicy` return the policy names.
- `AddPlatformIdentityJwt(Action<PlatformIdentityJwtOptions>)` — registers JWT options validated on startup; disabled until `Enabled` is set.

### Claim projection

- `HttpCurrentUserAccessor` — projects configured subject (default `ClaimTypes.NameIdentifier`, falls back to `sub`), email (default `ClaimTypes.Email`, falls back to `email`), tenant, role, and permission claims into `CurrentUser`; returns `CurrentUser.Anonymous` for unauthenticated requests; roles and permissions are deduplicated case-insensitively.
- `PlatformIdentityOptions` — configurable `SubjectClaimType`, `EmailClaimType`, `TenantClaimType`, `PermissionClaimType`, and `AuthenticationScheme`.

### Session service

- `IIdentitySessionService` / `IdentitySessionService` — host-level session operations over the registered `ISessionStore`, preserving `IdentityProviderResult<T>` outcomes and reporting `ProviderUnavailable` when no store is registered.

### Audit and validation

- `PlatformPermissionHandler` — succeeds for authenticated subjects carrying the permission and records a denied `IdentityAuditEvent` (`authorization.denied`) to a registered `IIdentityAuditHook` without token contents; `IAuthorizationDecisionAuditor` continues to receive the normalized decision.
- `PlatformIdentityJwtOptions` — `Enabled`, `Issuer`, `Audience`, `SigningKey`. `GetDiagnosticName()` returns a redacted view that never includes the signing key. Startup validation fails fast with a secret-free message when `Enabled` and any of `SigningKey`/`Issuer`/`Audience` is missing.

### Lifecycle integration

- `AddPlatformIdentityLifecycle(IServiceCollection)` — registers the `IIdentityLifecycleCoordinator` composition, the default `DefaultRefreshTokenService`, and a `MissingRefreshTokenService` / `MissingPasswordRecoveryService` / `MissingTwoFactorService` / `MissingImpersonationService` for any contract the application has not yet wired. The extension throws when the application calls a refresh-token method without an `IRefreshTokenStore`.
- `MapPlatformRefreshTokenRotation()` / `MapPlatformRefreshTokenRevocation()` / `MapPlatformPasswordRecoveryInitiation()` / `MapPlatformPasswordRecoveryCompletion()` / `MapPlatformTwoFactorChallenge()` / `MapPlatformTwoFactorVerification()` / `MapPlatformImpersonationStart()` / `MapPlatformImpersonationEnd()` — minimal-API helpers that route the lifecycle operations to the registered services. The helpers reuse the consumer's authentication scheme and claim projection; they never replace them.

## Platform.Identity.EntityFrameworkCore

Optional schema-independent EF Core identity store contracts. Depends on `Platform.Identity.Contracts` and `Platform.Persistence.EfCore`; references `Microsoft.EntityFrameworkCore`. No platform user entity, schema, migration, or business relationship is supplied.

- `IIdentityStore` — application-owned lookup/link operations returning normalized outcomes.
- `IdentityDbContextAdapter` — abstract `DbContext` base the application derives to configure its own identity entities.
- `AddPlatformIdentityStore<T>()` — `TryAdd` seam for the application-owned `IIdentityStore`.

## Platform.Identity.Testing

Deterministic identity and authorization test providers. Depends on `Platform.Identity.Contracts` and `Platform.Authorization`.

- `FakeCurrentUserAccessor`, `FakeCredentialVerifier`, `FakeExternalIdentityProvider`, `FakeVerificationProvider` — deterministic in-memory providers keyed by fixture input.
- `RecordingAuthorizationDecisionAuditor` — collects `AuthorizationDecision` records for assertions.
- `InMemoryRefreshTokenStore` — deterministic, linearizable refresh-token store with `Replayed` / `Revoked` family revocation. Not for production; consumers must provide a hashed-at-rest, multi-instance-capable store.
- `FakePasswordRecoveryService` — same `Succeeded` outcome for known and unknown subjects, with `Replayed` on second completion. `InitiatedChallenges` records every issued challenge.
- `FakeTwoFactorService` — opaque `TwoFactorChallenge` with configurable `AcceptedCode`. `IssuedChallenges` records every issued challenge.
- `FakeImpersonationService` — delegates to a swappable `IImpersonationPolicy`; defaults to `DenyAllImpersonationPolicy` so the platform's fail-closed behavior is preserved in tests.
- `AllowImpersonationPolicy` / `DenyAllImpersonationPolicy` — out-of-the-box policies.
- `RecordingIdentityAuditHook` — collects every `IdentityAuditEvent` for assertions.

# Platform.Realtime

Opt-in, application-owned realtime transports over provider-neutral contracts.
`Platform.Realtime` is framework-neutral (no ASP.NET Core / SignalR / Redis
dependency) and defines connection lifecycle, authorization, tenant routing,
bounded payloads, provider status, and delivery-disclosure contracts.
`Platform.Realtime.AspNetCore` adds the SSE and SignalR adapters and depends only
on the `Microsoft.AspNetCore.App` framework reference; the backplane is
application-owned and optional.

## Platform.Realtime (contracts)

### Connection options

- `RealtimeConnectionOptions` — `MaxConcurrentConnections` (default `1000`), `MaxPayloadBytes` (default `65536`), `ConnectionIdleTimeout` (default 5 min), `HeartbeatInterval` (default 30 s), `AllowCrossTenantBroadcast` (default `false`), `SectionName` constant `"Realtime"`. `Validate()` returns human-readable failures.

### Authorization and tenant routing

- `IRealtimeConnectionAuthorizer` — `AuthorizeAsync(request, ct)`; the platform never connects a client without a positive `RealtimeAuthorizationResult`. `DenyAllRealtimeAuthorizer` is the fail-closed default.
- `RealtimeConnectionRequest` — inbound connection description (connection id, `CallerContext`, optional metadata).
- `IRealtimeTenantRouter` — `IsRouteAllowedAsync(route, ct)`; `DenyCrossTenantRouter` (allow only when target tenant equals caller tenant, or both unset) is the fail-closed default.
- `RealtimeTenantRoute` — candidate delivery route (target tenant, caller).

### Messages and status

- `RealtimeMessage.Create(maxPayloadBytes, channel?, targetTenantId?, targetSubjectId?, payloadText?, payload?)` — bounded, validated message. `GetPayloadByteCount()`.
- `IRealtimeProviderStatus` / `RealtimeProviderStatus` — safe, topology-free transport health (transport name, availability, backplane flag). Never exposes connection strings or credentials.
- `RealtimeResyncRequest` and `RealtimeDelivery.IsDurable` (`false`) — the non-durable delivery disclosure. Replay/resync is application-owned.

### Registration

- `AddPlatformRealtime(IServiceCollection)` and the `Action<RealtimeConnectionOptions>` overload — bind options and register `IClock` when no implementation is present. The application supplies `IRealtimeConnectionAuthorizer` and `IRealtimeTenantRouter`.

## Platform.Realtime.AspNetCore (adapters)

### SSE

- `AddPlatformRealtimeAspNetCore(...)` — registers the shared contracts, fail-closed defaults, the connection limiter, and the provider status. Also registers `IRealtimeSseSource` resolution expectation; applications supply the source, caller resolver, authorizer, and router.
- `MapPlatformRealtimeSse(pattern)` — maps a Server-Sent Events stream that authorizes (reject `401` when denied), enforces the connection limit (reject `503` when full), sets safe `text/event-stream` headers, streams the application `IRealtimeSseSource`, emits heartbeats, and releases the connection slot on disconnect/cancellation. Each written message is tenant-routed and payload-bounded by the platform `SseMessageSink`.

### SignalR

- `AddPlatformRealtimeSignalR(...)` — registers SignalR with a connection-authorization `IHubFilter` and a tenant-routing-aware `RealtimeHubBase`. The backplane is an application-owned seam (`RealtimeSignalROptions.ConfigureBackplane`); without it the host runs in-process (no-backplane mode).
- `RealtimeHubBase` — optional base hub that authorizes on connect (fail-closed) and exposes a tenant-routing-aware `SendToTenantAsync`.

### Status and resync

- `MapPlatformRealtimeStatus(pattern = "/realtime/status")` — exposes safe transport health.
- `MapPlatformRealtimeResync(pattern = "/realtime/resync")` — forwards a client `RealtimeResyncRequest` to the application `IRealtimeResyncHandler`; `404` when no handler is registered.

See [`platform-realtime.md`](platform-realtime.md) for the migration guide and the
non-durable delivery contract.

# Platform.Starter

`Platform.Starter` composes the opt-in web, identity, administration, billing, and mailing
boundaries. It owns no application persistence or provider implementation. See
[`platform-starter.md`](platform-starter.md).

# Platform UI

`Platform.UI.Razor` ships token-backed Razor static assets. The token source and generated
React artifacts live under `ui/`: `@platform/design-tokens`, `@platform/react-ui`, and
`@platform/react-shell`. React and Razor share semantic tokens and state conventions while
retaining independent implementations.

## Distribution and conformance

The three npm packages stay at `0.1.0` with digest pinning, and their packed bytes are
committed in `ui/feed/*.tgz` beside a `ui/feed/feed.json` digest manifest. A repository
outside this one consumes the layer by committing those tarballs into its own tree and
declaring a `file:` dependency specifier plus a one-file `ui-conformance.json`; no sibling
checkout, absolute path, environment variable or registry is involved. `ui/tokens/tokens.json`
is the single source of every token value, and the generated CSS additionally defines the
canonical `.visually-hidden` primitive (nine properties, no `!important`) that 15
repositories had each written themselves.

- `node ui/scripts/generate-tokens.mjs` regenerates `dist/tokens.css` and `dist/tokens.ts`
  from the token source by walking the DTCG tree.
- `node ui/scripts/verify-ui.mjs` is the oracle and is **wired into
  `scripts/quality-gate.sh`**: it fails with a named diagnostic on a dropped token
  (`ui-token-dropped`), an undeclared literal such as a hard-coded dark-theme value
  (`ui-orphan-literal`), an unresolvable alias (`ui-alias-unresolved`), or a missing or
  duplicated canonical primitive.
- `scripts/check-ui-conformance.sh` scans the workspace and fails a repository that
  declares a shared UI primitive itself (`ui-divergent-primitive`), ships a user interface
  with no declaration (`ui-conformance-undeclared`), or introduces a new non-scale
  breakpoint (`ui-breakpoint-undeclared`). The 15 known divergent repositories, the
  undeclared-UI repositories and the existing breakpoint values are frozen in
  `ui/legacy-divergent.json` so the check fails the *next* divergence without blocking
  today's.
- `scripts/test-ui-distribution.sh` exercises all of the above on fixtures, including a
  real offline `npm install` at an unrelated path.

See [`ui/README.md`](../ui/README.md) for the mechanism, the three refused dependency
path forms, the version policy and the known boundaries. `Platform.UI.Razor` is not in
`scripts/check-public-api.sh`'s six-package scan, a deliberate and recorded coverage gap:
`eng/public-api-baseline.txt` is unchanged by this capability.

# Platform.Notifications

`Platform.Notifications` provides channel-neutral email/SMS intents, delivery outcomes,
bounded retries, idempotency integration, and job scheduling helpers. `Platform.Notifications.Testing`
contains the deterministic in-memory provider. Providers and templates remain application-owned;
see [`platform-notifications.md`](platform-notifications.md).

# Platform.Auditing

`Platform.Auditing.Contracts` provides provider-neutral audit event, scope, enrichment,
masking, sink, retention, dead-letter, and failure-policy contracts. `Platform.Auditing.AspNetCore`
adds opt-in request, exception, and security capture through a `FrameworkReference`-only
middleware. `Platform.Auditing.EfCore` exposes an opt-in `SaveChangesInterceptor` that captures
changes for application entities implementing `IAuditedEntity`. The platform ships no durable
audit store, schema, retention schedule, or migration. See
[`platform-auditing.md`](platform-auditing.md).

## Platform.Auditing.Contracts

Framework-neutral; targets `net10.0`. Depends on `Platform.Core` and the four
`Microsoft.Extensions.*` abstractions only. Does not reference ASP.NET Core, EF Core, Stripe,
or application projects.

### Events

- `AuditEvent` — immutable, normalized record with `Action`, `Category`, `Outcome`,
  `Severity`, `OccurredAt` (UTC `DateTimeOffset`), optional `CorrelationId`/`TenantId`/
  `SubjectId`/`Source`, and bounded `Metadata` (`IReadOnlyDictionary<string, string>` already
  masked before assignment). `Create(action, category, outcome, occurredAt, severity)` factory
  and `With*` copy helpers; `Validate()` returns human-readable errors.
- `AuditOutcome` enum — `Unknown`, `Success`, `Failure`, `Error`, `Denied`.
- `AuditSeverity` enum — `Information`, `Warning`, `Error`.

### Pipeline contracts

- `IAuditRecorder` — `RecordAsync(event, ct)`; enriches, masks, and dispatches to the sink under
  the configured failure policy. The single entry point used by the HTTP and EF adapters.
- `IAuditSink` — `RecordAsync(event, ct)`; the final destination. Applications own the durable
  implementation; the platform ships `InMemoryAuditSink` (bounded, capacity 1024) as the default.
- `IAuditMasker` — `Mask(key, value)` returns the stored value; `DefaultAuditMasker` redacts
  values whose key name matches a sensitive pattern (password, token, secret, ssn, cvv, card,
  privatekey, clientsecret, authorization, otp, …; case-insensitive, separators ignored) and
  returns `[REDACTED]`. `DefaultAuditMasker.ToStringValue(value)` converts non-strings
  invariant-culturally for masking.
- `IAuditEnricher` — `Enrich(event)` returns an enriched copy; applied in registration order and
  must not throw. `NoOpAuditEnricher` is the default.
- `IAuditRetentionPolicy` — `ShouldRetain(event)`; `RetainAllAuditRetentionPolicy` is the default.
- `IAuditDeadLetterSink` — `RecordAsync(event, reason, ct)` for failed publishes;
  `InMemoryAuditDeadLetterSink` and `NoOpAuditDeadLetterSink` are provided.
- `IAuditProviderStatusSource` — `GetStatus()` reporting safe availability;
  `DefaultAuditProviderStatusSource` returns `memory`/`available`.
- `IAuditedEntity` — marker interface; only entities implementing it are inspected by the EF
  interceptor.

### Options

- `AuditOptions` — `SectionName = "Auditing"`; `FailurePolicy` (`FailOpen` default),
  `PublishMode` (`Synchronous` default; `BoundedAsync` uses a bounded channel of `BoundedCapacity`
  = 1024, drop-on-full), `MaxMetadataEntries` (64), `MaxMetadataValueLength` (4096),
  `EnableEntityCapture` (true), `CaptureRequestBodyPreview` (false), `ExcludedCategories` /
  `EnabledCategories` (mutually exclusive). `IsCategoryEnabled(category)` and `Validate()`.

### Registration

- `AddPlatformAuditing(IServiceCollection)` and the `Action<AuditOptions>` overload — bind and
  validate options, register `IClock` (`SystemClock`) when no clock is present, and `TryAdd` the
  default masker, enricher, sink, dead-letter sink, retention policy, provider status source, and
  recorder. Applications replace any of these before or after the call.

## Platform.Auditing.AspNetCore

Optional ASP.NET Core capture adapter. Depends on `Platform.Core`, `Platform.Auditing.Contracts`,
and the `Microsoft.AspNetCore.App` framework reference; no third-party packages. Does not reference
EF Core, Stripe, or application projects.

### Options

- `AuditAspNetCoreOptions` — `SectionName = "Auditing:AspNetCore"`; `Enabled` (true),
  `ExemptPathPrefixes` (default `/health`, `/healthz`, `/ready`, `/live`, `/alive`, `/metrics`),
  `SubjectHeaderName` (`X-Audit-Subject`), `TenantHeaderName` (`X-Audit-Tenant`),
  `CorrelationHeaderName` (`X-Correlation-Id`), `CaptureRequestBodyPreview` (false),
  `MaxBodyPreviewBytes` (65536), `BodyPreviewLimit` (1024), `SecurityStatusCodes` (401, 403).
  `IsExempt(path)` and `IsSecurityStatus(statusCode)`.

### Capture

- `IAuditSubjectResolver` / `AuditSubjectResolution` — resolves subject and tenant;
  `HeaderAuditSubjectResolver` reads the configured headers and is the default.
- `AuditExceptionClassifier.Classify(Exception)` — maps to a safe `AuditExceptionClassification`
  (`validation`, `not_found`, `conflict`, `dependency`, `timeout`, `server_error`, `unknown`)
  without exposing the message, stack, or inner details. `SocketException`/`HttpRequestException`/
  `DbException` → dependency; `TimeoutException`/`TaskCanceledException`/`OperationCanceledException`
  → timeout.
- `AuditMiddleware` — records an `http.request` event (`AuditOutcome.Error` + `AuditSeverity.Error`
  on exceptions), derives `security`/denied severity for configured status codes, and is fail-open:
  a throwing sink never fails the request. Registered via `UsePlatformAuditing(IApplicationBuilder)`.

### Registration

- `AddPlatformAuditingAspNetCore(IServiceCollection)` and the `Action<AuditAspNetCoreOptions>`
  overload — register the contracts pipeline, validate options, and `TryAdd` the subject resolver
  and middleware.
- `UsePlatformAuditing(IApplicationBuilder)` — adds the capture middleware to the request pipeline.

## Platform.Auditing.EfCore

Optional EF Core change-capture adapter. Depends on `Platform.Core`, `Platform.Auditing.Contracts`,
and `Microsoft.EntityFrameworkCore` (+ `Microsoft.EntityFrameworkCore.Relational`) and the four
`Microsoft.Extensions.*` abstractions; targets `net10.0`. Does not reference ASP.NET Core, Stripe,
or application projects. It does not own migrations or select a transport.

### Interceptor

- `AuditingSaveChangesInterceptor` — `SaveChangesInterceptor` capturing `Added`/`Modified`/`Deleted`
  entries for `IAuditedEntity` entities. Scalar property changes (strings and value types, not
  navigation references) are masked by key name via `IAuditMasker` and published as
  `entity.created` / `entity.updated` / `entity.deleted` events (`entity.change.{Property}` =
  `set:`, `removed:`, or `old->new`). It is fail-open: disabled capture or a publishing failure is
  logged and skipped, never blocking the save. Registered through `AddPlatformAuditingEfCore`.

### Registration

- `AddPlatformAuditingEfCore(IServiceCollection)` — register the contracts pipeline and
  `TryAddSingleton<ISaveChangesInterceptor, AuditingSaveChangesInterceptor>()`. The application
  wires the interceptor into its `DbContext` through `options.AddInterceptors(sp.GetRequiredService<ISaveChangesInterceptor>())`.

# Platform.Webhooks

`Platform.Webhooks.Contracts` provides provider-neutral inbound verification, replay
suppression, outbound subscriptions, delivery attempts, retry options, and safe failure
metadata. `Platform.Webhooks.AspNetCore` adds the `HttpRequest` reader and a default
`HttpClient`-backed sender. `Platform.Webhooks.EfCore` exposes inbox and delivery entity
configurations for the application's `DbContext`. See [`platform-webhooks.md`](platform-webhooks.md).

# Platform.Observability

`Platform.Observability` provides opt-in host observability registration with safe
correlation, logging enrichment, tracing, metrics, and provider-status contracts. It depends
on `Platform.Core`, `Platform.Web.Telemetry`, the `Microsoft.AspNetCore.App` framework
reference, and the four `Microsoft.Extensions.*` abstractions. It does not reference EF Core,
PostgreSQL, Redis, AWS SDK, OpenTelemetry, Serilog, or Stripe. The platform owns no
exporter, collector, dashboard, Serilog sink configuration, or product log schema.

- `PlatformObservabilityOptions` — `ApplicationName`, `ApplicationVersion`,
  `EnvironmentName`, `CorrelationHeader`, `AcceptIncomingCorrelationHeader`,
  `MaxCorrelationIdLength`, `MaxTagLength`, `MaxOperationLength`,
  `TruncateOversizedValues`, `EnableRequestEnrichment`, `EnableProviderEnrichment`,
  `EnableHostLifecycle`, `RecordRequestBodyPreview`. Validated at registration time.
- `PlatformObservabilityNames` — stable activity source, meter, operation, tag, and
  outcome names. Do not rename.
- `IPlatformObservabilityRedactor` / `DefaultPlatformObservabilityRedactor` —
  redaction contract and `[REDACTED]` placeholder.
- `PlatformObservabilitySafeValuePolicy` — `BoundTag` (length-bound only) for
  already-safe values; `RedactTag` and `RedactOperation` (redact + bound) for
  known-sensitive values; `RequireOperation` and `RequireCorrelationId` validators.
- `PlatformDiagnostics` — framework-owned `ActivitySource` and `Meter` accessors plus
  the latest observed duration and counter helpers.
- `IPlatformActivityRecorder` / `DefaultPlatformActivityRecorder` — bounded
  activities for host, request, and provider call paths.
- `IPlatformObservabilityProviderStatusSource` /
  `DefaultPlatformObservabilityProviderStatusSource` — safe availability reporting.
- `PlatformObservabilityProviderCall` / `IPlatformObservabilityProviderRecorder` /
  `DefaultPlatformObservabilityProviderRecorder` — provider call enrichment bridge.
- `IPlatformCorrelationAccessor` / `HttpPlatformCorrelationAccessor` — correlation
  identifier accessor.
- `ICorrelationIdGenerator` / `DefaultCorrelationIdGenerator` — bounded identifier
  generation and normalization.
- `PlatformObservabilityCorrelationMiddleware` — echoes the bounded correlation
  identifier on the response and emits the request enrichment activity.
- `PlatformObservabilityHostLifetime` — `IHostedLifecycleService` that records host
  startup and shutdown activities and metrics.
- `AddPlatformObservability(IServiceCollection)` and the `Action<...>` overload.
- `UsePlatformObservability(IApplicationBuilder)` — adds the correlation middleware.

See [`platform-observability.md`](platform-observability.md) for adoption and migration
guidance.

# Platform.Web edge packages

Four small, independently adoptable packages extend `Platform.Web` with adjacent web
capabilities. Each is optional; consumers adopt one at a time and the core runtime
behaviour is unchanged. See [`platform-web-edge.md`](platform-web-edge.md) for the
adoption guide.

## Platform.Web.Telemetry

Framework-neutral redaction-safe web telemetry names, option-validation helpers, and a
structured log sink. Depends only on `Platform.Core`, `Microsoft.Extensions.DependencyInjection.Abstractions`,
`Microsoft.Extensions.Logging.Abstractions`, `Microsoft.Extensions.Options`, and
`System.Diagnostics.DiagnosticSource`; targets `net10.0`. Does not reference ASP.NET Core,
EF Core, Polly, Swashbuckle, or NSwag.

- `PlatformWebTelemetryNames` — stable activity source, operation, and metric names
  (`RequestActivitySource`, `ProviderActivitySource`, `RequestOperation`,
  `ProviderOperation`, `RequestCountMetric`, `ProviderCountMetric`, plus stable
  tag keys).
- `PlatformWebTelemetryOptions` — `ApplicationName`, `MaxTagLength`,
  `MaxOperationLength`, `TruncateOversizedValues`. Validated at registration time.
- `PlatformWebRequestEvent` / `PlatformWebProviderEvent` — typed event records for
  requests and provider calls. `Default` constructor enforces non-empty operation,
  valid status code, and non-negative duration.
- `IPlatformWebTelemetryRedactor` / `DefaultPlatformWebTelemetryRedactor` — redaction
  contract and `[REDACTED]` placeholder.
- `PlatformWebTelemetrySafeValuePolicy` — bounds and redacts tag and operation values.
- `IPlatformWebTelemetry` / `DefaultPlatformWebTelemetry` — structured log sink.
- `PlatformOptionsValidator` and `AddValidatedOptions<T>()` — shared option
  validation helper for any platform options type.
- `AddPlatformWebTelemetry(IServiceCollection)` and the `Action<...>` overload — register
  the redactor, the default telemetry sink, and validated options.

## Platform.Web.Cors

Optional ASP.NET Core CORS configuration and production-time validation. Depends on
`Platform.Core`, `Platform.Web.Telemetry`, `Microsoft.AspNetCore.App` (via
`FrameworkReference`); targets `net10.0`. Does not reference EF Core, Stripe, or
application projects.

- `PlatformWebCorsPolicyOptions` — per-policy `Name`, `AllowedOrigins`,
  `AllowedHeaders`, `ExposedHeaders`, `AllowedMethods`, `AllowCredentials`,
  `PreflightMaxAge`. `Validate(environmentName)` rejects wildcard origins with
  credentials, wildcard origins in production, missing origins in production, and
  non-absolute or non-HTTPS origins outside development.
- `PlatformWebCorsOptions` — `Environment`, `Policies` collection, `RoutePrefix`.
  `Validate()` enforces non-empty policies, unique names, and per-policy validation.
- `AddPlatformWebCors(IServiceCollection)`, the `Action<...>` overload, and the
  `IHostEnvironment` overload — register validated options, telemetry, and named
  CORS policies.
- `UsePlatformWebCors()` and `UsePlatformWebCors(policyName)` — call the underlying
  `UseCors` with the supplied policy.

## Platform.Web.Resilience

Optional `HttpClient` resilience conventions (retry, timeout, circuit breaker) with
bounded defaults. Depends on `Platform.Core`, `Platform.Web.Telemetry`,
`Microsoft.AspNetCore.App`, and `Microsoft.Extensions.Http`; targets `net10.0`. Does
not reference EF Core, Polly, or application projects.

- `PlatformHttpResilienceOptions` — `AttemptTimeout` (5s), `MaxRetryAttempts` (3),
  `RetryBaseDelay` (200ms), `CircuitBreakerFailureRatio` (0.5),
  `CircuitBreakerSamplingDuration` (30s), `CircuitBreakerMinimumThroughput` (10),
  `CircuitBreakerBreakDuration` (30s), `IdempotentMethods`, `EmitRetryAttemptHeader`.
  Validated at registration time.
- `HttpResilienceDecision` — `None`, `Retried`, `TimedOut`, `CircuitBroken`.
- `HttpResilienceEvent` — typed event with `Decision`, `Operation`, `Attempt`,
  `StatusCode`.
- `IHttpResilienceTelemetry` / `DefaultHttpResilienceTelemetry` — bridge to the
  platform web telemetry sink.
- `PlatformHttpResilienceHandler` — `DelegatingHandler` that retries only
  idempotent methods (`GET`, `HEAD`, `OPTIONS`, `PUT`/`DELETE` with `If-Match`,
  and any explicitly opted-in method), emits `X-Retry-Attempt` on each attempt,
  applies a per-attempt timeout, and trips a circuit breaker after
  `CircuitBreakerMinimumThroughput` failures at or above the failure ratio.
- `PlatformHttpCircuitOpenException` — thrown while the breaker is open.
- `AddPlatformHttpResilience(IServiceCollection)` and the `Action<...>` overload.
- `AddPlatformHttpResilience(IHttpClientBuilder)` — adds the handler to a named
  client.

## Platform.Web.OpenApi

Optional OpenAPI document registry and explicit mapping helper. Depends on
`Platform.Core`, `Platform.Web.Telemetry`, and `Microsoft.AspNetCore.App` (via
`FrameworkReference`); targets `net10.0`. Does not reference Swashbuckle, NSwag,
EF Core, or application projects.

- `PlatformWebOpenApiOptions` — `Documents`, `RoutePrefix` (`/openapi`).
  `Validate()` enforces unique names, absolute paths, and bounded options.
- `PlatformWebOpenApiDocumentOptions` — `Name`, `Path`, `ContentType`, `Title`,
  `OpenApiVersion`.
- `IPlatformOpenApiDocumentProvider` — `SupportedDocuments` plus
  `GetDocument(name)`. The platform does not own a specific OpenAPI
  implementation; applications supply the JSON.
- `IPlatformOpenApiDocumentRegistry` / `PlatformOpenApiDocumentRegistry` — aggregate
  providers and resolve by name. Returns `null` for unknown names.
- `Providers.StaticPlatformOpenApiDocumentProvider` — emits a minimal placeholder
  document for testing and bootstrapping.
- `AddPlatformWebOpenApi(IServiceCollection)` and the `Action<...>` overload.
- `AddPlatformOpenApiDocument(PlatformWebOpenApiDocumentOptions)` — convenience
  registration of a static document provider.
- `MapPlatformOpenApiDocument(name)` — maps `/openapi/{name}.json`. Authorization
  metadata applied with `RequireAuthorization()` is preserved.
- `MapPlatformOpenApiDocuments()` — maps every registered document.

## Platform.Web.Versioning

Optional ASP.NET Core API versioning and API Explorer conventions. Depends on
`Platform.Core`, `Microsoft.AspNetCore.App` (via `FrameworkReference`), and the
centrally managed `Asp.Versioning.Http` and `Asp.Versioning.Mvc.ApiExplorer`
packages; targets `net10.0`. Does not reference Swashbuckle, NSwag, EF Core, or
application projects.

- `PlatformWebVersioningOptions` — `DefaultMajor` (1) / `DefaultMinor` (0),
  `AssumeDefaultVersionWhenUnspecified` (true), `ReportApiVersions`,
  `RouteConstraintName` (`apiVersion`), `GroupNameFormat` (`'v'VVV`), `Reader`
  (`UrlSegment` default), `HeaderName` (`X-Api-Version`), `QueryParameterName`
  (`api-version`). `Validate()` rejects negative versions, missing
  reader-specific names, and empty format strings.
- `PlatformVersionReaderKind` — `UrlSegment`, `Header`, `QueryString`,
  `MediaType`, `Composite`.
- `IPlatformVersioningDefaultsProvider` / `PlatformVersioningDefaultsProvider`
  — exposes the configured `DefaultApiVersion` and
  `AssumeDefaultVersionWhenUnspecified` flag for tests and applications.
- `AddPlatformWebVersioning(IServiceCollection)` and the `Action<...>` overload
  — registers the platform options, the Asp.Versioning services, and the API
  Explorer services.
- `EnablePlatformApiVersionBinding(IApiVersioningBuilder)` — opt-in helper
  that turns on `IApiVersioningBuilder.EnableApiVersionBinding()` for
  consumers that bind `ApiVersion` in minimal API parameter lists.
- `MapPlatformApiExplorerDescriptions(Action<IEndpointConventionBuilder, ApiVersionDescription>)`
  — groups descriptions by `GroupNameFormat` and routes each group under
  `/v<groupName>`. The platform never generates OpenAPI documents; consumers
  that need JSON should still register an `IPlatformOpenApiDocumentProvider`
  via `Platform.Web.OpenApi`.

## Platform.FeatureManagement

Optional ASP.NET Core feature-flag integration. Depends on `Platform.Core`,
`Platform.Web.Telemetry`, the `Microsoft.AspNetCore.App` framework reference, and
`Microsoft.FeatureManagement`; targets `net10.0`. Does not own feature names,
rollout state, billing plans, tenant records, EF Core, or Polly. The host owns the
`FeatureManagement` configuration section and the rollout rules.

### Context

- `FeatureContext` — record carrying optional `TenantId`, `Subject`, and bounded
  `Properties` (no secrets or rollout state).
- `IFeatureContextResolver` — `ResolveAsync(cancellationToken)` returns the
  application-provided `FeatureContext`. `DefaultFeatureContextResolver` returns an
  empty context; consumers replace it before registration to supply tenant/subject
  state.

### Filters and gating

- `PlatformTenantFeatureFilter` — `[FilterAlias("PlatformTenant")]` filter that
  reads `TenantId` from the resolver and compares it against the per-feature
  `AllowedTenants` parameter; returns `false` when the tenant is empty. Registered
  automatically when the integration is enabled.
- `FeatureGateEndpointFilter` and
  `RequireFeature(this RouteHandlerBuilder, string featureName)` — gate an endpoint
  behind a flag. On a disabled feature the filter returns a safe `ProblemDetails`
  built from `FeatureManagementOptions`; no flag or rollout state leaks.

### Options

- `FeatureManagementOptions` — `Enabled` (default `true`), `SectionName` (default
  `"FeatureManagement"`), `DisabledStatusCode` (default `404`), `DisabledTitle`
  (default `"Feature disabled"`). `Validate()` bounds the status code to 100–599 and
  rejects empty section or title.

### Registration

- `AddPlatformFeatureManagement(IServiceCollection, IConfiguration)` and the
  `Action<FeatureManagementOptions>` overload — register validated options and the
  platform web telemetry sink, register the tenant filter when `Enabled`, and
  register `IFeatureContextResolver` (default empty). The host owns the feature
  definition section.

## Platform.Http.Resilience

Optional outbound HTTP resilience integration. Framework-neutral; targets `net10.0`.
Depends on `Platform.Core`, `Platform.Web.Telemetry`, `Microsoft.Extensions.Http`,
and `Microsoft.Extensions.Http.Resilience`. Does not reference ASP.NET Core, EF
Core, `Microsoft.FeatureManagement`, or Polly directly, and adds no provider SDKs.
Each named `HttpClient` receives its own bounded pipeline; applications own provider
selection and per-service overrides.

### Options

- `PlatformHttpResilienceOptions` — `Enabled` (default `true`), `MaxRetryAttempts`
  (default `3`, bounded 1–10), `RetryBaseDelay` (default `500ms`), `AttemptTimeout`
  (default `5s`), `TotalTimeout` (default `30s`), `CircuitBreakerFailureRatio`
  (default `0.5`), `CircuitBreakerSamplingDuration` (default `30s`),
  `CircuitBreakerMinimumThroughput` (default `10`), `CircuitBreakerBreakDuration`
  (default `30s`), `MaxConcurrentCalls` (default `100`), `MaxQueueLength` (default
  `1000`), `IdempotentMethods` (default empty; the safe set `GET`/`HEAD`/`OPTIONS`/
  `TRACE` is always included). `Validate()` enforces every bound and rejects empty
  idempotent entries.

### Telemetry

- `PlatformHttpResilienceDecision` — `None`, `Retried`, `CircuitOpen`,
  `ConcurrencyRejected`, `Cancelled`.
- `PlatformHttpResilienceEvent` — bounded record (`Decision`, `Operation`, `Attempt`,
  `StatusCode`); `Operation` never includes URLs, secrets, or headers.
- `IPlatformHttpResilienceTelemetry` / `DefaultPlatformHttpResilienceTelemetry` —
  record sink; the default is a no-op that consumers replace with an
  application-owned sink.

### Registration

- `AddPlatformHttpResilience(this IHttpClientBuilder)` and the
  `Action<PlatformHttpResilienceOptions>` overload — bind validated options,
  register the telemetry sink (default no-op) and `PlatformHttpResilienceHandler`,
  and add the handler as an HTTP message handler. When `Enabled` is `false` the
  handler is a pass-through.

### Behavior

- The pipeline applies, in order: a total timeout, retry (exponential back-off with
  jitter, idempotent methods only by default), a per-attempt timeout, a circuit
  breaker (trips after the configured failure ratio and minimum throughput), and a
  concurrency limiter (bounded queue). The HTTP method is carried through a
  resilience context property so idempotency classification is reliable. Caller
  cancellation is preserved and recorded without being retried. Non-idempotent
  methods are never replayed unless explicitly added to `IdempotentMethods`.

# Platform.Web.Edge.Tests

`tests/Platform.Web.Edge.Tests` exercises all four edge packages:

- Telemetry contract tests: stable names, default option validation, safe-value
  policy, default telemetry registration, and validator helper.
- CORS option tests: wildcard+credentials rejection, production origin
  requirement, localhost exception, duplicate policy names, empty collection, and
  runtime validation.
- CORS integration tests: allowed origin receives the header, disallowed origin
  does not, and wildcard origin is served without credentials.
- Resilience option tests: defaults, bounded ranges, and runtime validation.
- Resilience handler tests: transient GET retries, POST is not retried by
  default, explicit idempotent override, successful response is not retried,
  retry attempt header is emitted, and circuit breaker opens after the threshold.
- OpenAPI integration tests: registered document is served, unregistered document
  is 404, multiple documents resolve independently.