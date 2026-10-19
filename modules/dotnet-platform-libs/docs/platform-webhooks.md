# Platform webhooks

`Platform.Webhooks` standardizes inbound verification, replay suppression, outbound delivery, and
SSRF-safe target validation. Provider-specific signature schemes, application event payloads, and
durable event application remain application responsibilities.

## Packages and layout

- `Platform.Webhooks.Contracts` — `Inbound`, `Outbound`, `Security`, `Common`, and
  `DependencyInjection` folders; framework-neutral contracts, in-memory stores, the default
  `HmacWebhookSignatureVerifier`, and the default `SsrfTargetValidator`.
- `Platform.Webhooks.AspNetCore` — opt-in helpers that capture raw `HttpRequest` bytes and sign
  outbound deliveries through an application-owned `HttpClient`. No provider-specific routes are
  registered.
- `Platform.Webhooks.EfCore` — opt-in `DbContext`-agnostic entity configurations and stores. The
  application owns the `DbContext` and migrations; the platform does not own a `DbSet<>` for
  subscriptions (they are value objects without settable properties).

## Inbound lifecycle

`WebhookInboundProcessor` orchestrates three independent decisions:

1. Verification via `IWebhookSignatureVerifier`. The default `HmacWebhookSignatureVerifier` accepts
   the raw body, a signature header, and an optional timestamp header. It rejects missing,
   malformed, or stale signatures, and never exposes the secret or payload in failures.
2. Secret resolution via `IWebhookSecretResolver`. The default `ConfigurationWebhookSecretResolver`
   keeps an in-memory dictionary keyed by `(provider, secret-key)`. Applications supply their own
   resolver for secret stores backed by configuration, key vaults, or other sources.
3. Replay suppression via `IWebhookInboxStore`. The default `InMemoryWebhookInboxStore` records
   `(provider, event-id)` and leases the message to a worker. A duplicate identifier returns
   `WebhookInboxClaimStatus.Duplicate` without re-running the handler.

The processor returns `WebhookInboundResult.Accepted`, `Duplicate`, `Busy`, or `Rejected` with a
safe `WebhookFailure`. Handler outcomes (`Succeeded`, `TransientFailure`, `PermanentFailure`)
schedule a retry or move the message to a terminal failure state.

## Outbound lifecycle

`WebhookOutboundDispatcher` validates the subscription, signs the payload, sends the request, and
records the resulting state.

- Subscriptions are application-owned `WebhookSubscription` value objects identified by
  `WebhookSubscriptionId`. The retry policy supports capped exponential back-off.
- The `SsrfTargetValidator` rejects non-absolute, non-HTTPS, loopback, private, and link-local
  targets by default. The `AllowLoopbackTargets` flag and the `TargetAllowList` allow-list
  override this for development or tightly scoped production deployments.
- The `IWebhookHttpSender` boundary keeps transport concerns out of the dispatcher. The default
  `HttpClientWebhookSender` (in `Platform.Webhooks.AspNetCore`) enforces the configured
  `DefaultOutboundTimeout` and maps response status codes to safe `WebhookFailure` categories.
- Delivery state is recorded through `IWebhookDeliveryStore`. 5xx, 408, and 429 responses
  schedule a retry using the subscription's `WebhookRetryPolicy`; all other 4xx responses are
  treated as permanent.

## Boundary with billing and durable eventing

- Billing provider adapters normalize provider events; the webhooks package supplies the inbound
  verification envelope and replay protection. The application receives a
  `WebhookVerificationRequest`, runs its own event-mapping handler, and records normalized
  business state.
- Outbound webhooks do not depend on the durable eventing dispatcher. They record the delivery
  state in their own store so that retries and observability are scoped to the webhook concern.
  The application may publish integration events using `Platform.Eventing` if it wants to fan
  out the same payload elsewhere.

## Adoption

- Verification keys, signing secrets, and event payloads are owned by the application. The
  packages provide composition seams (`IWebhookSecretResolver`, `IWebhookSubscriptionStore`,
  `IWebhookDeliveryStore`) that applications replace with their own implementations.
- The ASP.NET Core package exposes `MapPlatformWebhookStatus` for a minimal status endpoint. It
  does not register provider-specific routes; applications map them inside their own endpoints
  using the provided `WebhookHttpRequestReader` and `WebhookInboundProcessor`.
- The EF Core package provides `IEntityTypeConfiguration<>` for the inbox and delivery entities.
  The application supplies the `DbContext` and migrations. No migrations or default tables are
  shipped.

## Failure and redaction

`WebhookFailure` carries a stable `Code`, a safe `Message`, and a `Transient` flag. Verifier,
SSRF, and HTTP failures are normalized to a small set of codes; secrets, raw payloads, and stack
traces are not exposed through the failure surface.
