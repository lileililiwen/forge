# Platform billing

`Platform.Billing.Contracts` is the provider-neutral boundary. It defines checkout, portal,
subscription lookup, cancellation, webhook normalization, plan catalogs, entitlement storage,
and usage contracts. Provider IDs and plan mappings remain application-owned through
`ProviderPlanReference`; no Stripe, Lemon Squeezy, invoice, tax, wallet, or merchant-of-record
dependency is included.

`Platform.Billing` contains the small orchestration layer:

```csharp
var result = await new BillingEventOrchestrator(processedEventStore, projector, clock)
    .ProcessAsync(new ProviderEventEnvelope(normalizedProviderEvent));
```

The orchestrator marks `(provider, event id)` before projection. Duplicates are acknowledged as
`Duplicate`; projectors reject events older than the latest event they have applied. Hosts own
the durable processed-event and entitlement schemas by implementing the public interfaces.

`FeatureAccessEvaluator` combines the existing normalized `FeatureCheck` decision with an
`IUsageMeter`, returning stable reasons, current usage, and limits. Expired periods return
`FeatureCheckReason.Expired`; canceled or otherwise inactive subscriptions return
`NotSubscribed`.

`Platform.Billing.Testing` provides deterministic in-memory entitlement, usage, and provider
fakes. Persistent stores and concrete provider adapters are intentionally deferred to separate
changes.

## Optional provider adapters

`Platform.Billing.Stripe` and `Platform.Billing.LemonSqueezy` are opt-in raw-HTTP adapters.
Register them explicitly with `AddPlatformStripe` or `AddPlatformLemonSqueezy`, and provide
application-owned `PlanCatalog` mappings for provider price or variant identifiers. Webhook
handlers pass the raw body and provider headers to `VerifyAndNormalizeWebhookAsync` before
calling `BillingEventOrchestrator`. Stripe uses `Stripe-Signature`; Lemon Squeezy uses
`X-Signature`. Invalid or malformed signatures never produce an event.

Both adapters expose `GetStatusAsync`. Missing credentials report `not_configured`; credentials
and response bodies are excluded from diagnostics. `ProviderFailureClassifier` classifies network
and timeout failures as transient. Tests can use a custom `HttpMessageHandler` and fake secrets,
so local development does not require provider accounts or live credentials.
