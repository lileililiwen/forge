using Platform.Billing;
using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Features;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Billing.Contracts.Plans;
using Platform.Core.Time;
using Platform.Billing.Testing;
using Platform.Billing.Contracts.Usage;
using Platform.Billing.Contracts.Providers;

namespace Platform.Billing.Tests;

public sealed class BillingBehaviorTests
{
    [Fact]
    public void Expired_subscription_denies_with_stable_expired_reason()
    {
        var now = new DateTimeOffset(2026, 2, 1, 0, 0, 0, TimeSpan.Zero);
        var subscription = new Subscription(SubjectKey.Create("u1"), PlanId.Create("pro"), SubscriptionStatus.Active, ProviderName.Create("test"), "sub1", now.AddDays(-30), now);
        var entitlement = new Entitlement(subscription.Subject, null, subscription, new HashSet<FeatureKey> { FeatureKey.Create("reports") }, null, now);

        var result = FeatureCheck.Evaluate(entitlement, FeatureKey.Create("reports"), now);

        Assert.Equal(FeatureCheckReason.Expired, result.Reason);
        Assert.False(result.IsAllowed);
    }

    [Fact]
    public void Plan_catalog_maps_application_plan_to_opaque_provider_reference()
    {
        var plan = new BillingPlan(PlanId.Create("pro"), "Pro", new Dictionary<FeatureKey, long> { [FeatureKey.Create("reports")] = 100 });
        var catalog = new PlanCatalog().Add(plan, new ProviderPlanReference(ProviderName.Create("test"), "price_opaque"));

        Assert.Equal("price_opaque", catalog.ReferenceFor(plan.Id, ProviderName.Create("test")).ProviderValue);
        Assert.Equal(100, catalog.FeaturesFor(plan.Id)[FeatureKey.Create("reports")]);
    }

    [Fact]
    public async Task Usage_limit_returns_explainable_limit_exceeded_decision()
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var feature = FeatureKey.Create("api.calls");
        var subscription = new Subscription(SubjectKey.Create("u1"), PlanId.Create("pro"), SubscriptionStatus.Active, ProviderName.Create("test"), "sub1", now.AddDays(-1), now.AddDays(30));
        var entitlement = new Entitlement(subscription.Subject, null, subscription, new HashSet<FeatureKey> { feature }, null, now);
        var meter = new InMemoryUsageMeter();
        meter.SetLimit(feature, 2);
        await meter.RecordAsync(subscription.Subject, feature, 3);

        var result = await new FeatureAccessEvaluator(meter, new FixedClock(now)).CheckAsync(entitlement, feature);

        Assert.Equal(FeatureCheckReason.LimitExceeded, result.Reason);
        Assert.Equal(3, result.CurrentUsage);
        Assert.Equal(2, result.Limit);
    }

    [Fact]
    public async Task Orchestrator_skips_duplicate_and_stale_events()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 10, 0, 0, 0, TimeSpan.Zero));
        var projector = new InMemoryBillingProjector();
        var store = new InMemoryProcessedEventStore();
        var orchestrator = new BillingEventOrchestrator(store, projector, clock);
        var newer = NewEvent("evt-new", clock.UtcNow);
        var older = NewEvent("evt-old", clock.UtcNow.AddDays(-1));

        Assert.Equal(BillingEventDecision.Applied, await orchestrator.ProcessAsync(new ProviderEventEnvelope(newer)));
        Assert.Equal(BillingEventDecision.Duplicate, await orchestrator.ProcessAsync(new ProviderEventEnvelope(newer)));
        Assert.Equal(BillingEventDecision.IgnoredStale, await orchestrator.ProcessAsync(new ProviderEventEnvelope(older)));
        Assert.Equal(newer.Id, projector.Latest!.Id);
    }

    [Fact]
    public void Canceled_subscription_denies_even_when_the_plan_contains_the_feature()
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var feature = FeatureKey.Create("reports");
        var subscription = new Subscription(SubjectKey.Create("u1"), PlanId.Create("pro"), SubscriptionStatus.Canceled, ProviderName.Create("test"), "sub1", now.AddDays(-1), now.AddDays(30));
        var result = FeatureCheck.Evaluate(new Entitlement(subscription.Subject, null, subscription, new HashSet<FeatureKey> { feature }, null, now), feature, now);

        Assert.Equal(FeatureCheckReason.NotSubscribed, result.Reason);
    }

    [Fact]
    public async Task In_memory_provider_keeps_provider_references_out_of_normalized_calls()
    {
        var provider = new InMemoryBillingProvider(ProviderName.Create("fake"), new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        var providerEvent = NewEvent("evt-1", new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        provider.RegisterWebhook("payload", providerEvent);

        var result = await provider.VerifyAndNormalizeWebhookAsync("payload", new Dictionary<string, string>());

        Assert.True(result.Succeeded);
        Assert.Equal(providerEvent, result.Event);
        Assert.Equal(ProviderName.Create("fake"), provider.Name);
    }

    [Fact]
    public async Task Provider_cancellation_returns_normalized_canceled_subscription()
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var subject = SubjectKey.Create("u1");
        var provider = new InMemoryBillingProvider(ProviderName.Create("fake"), new FixedClock(now));
        provider.SetSubscription(new Subscription(subject, PlanId.Create("pro"), SubscriptionStatus.Active, ProviderName.Create("fake"), "sub1", now, now.AddDays(30)));

        var canceled = await provider.CancelSubscriptionAsync(new CancellationRequest(subject));

        Assert.Equal(SubscriptionStatus.Canceled, canceled!.Status);
    }

    private static ProviderEvent NewEvent(string id, DateTimeOffset occurredAt) =>
        new(ProviderEventId.Create(id), ProviderName.Create("test"), "subscription.updated", occurredAt, "{\"subject\":\"u1\"}", SubjectKey.Create("u1"));

    private sealed class InMemoryProcessedEventStore : IProcessedEventStore
    {
        private readonly HashSet<(ProviderName, ProviderEventId)> _seen = [];
        public Task<ProcessedEventDecision> MarkProcessedAsync(ProcessedEvent processedEvent, CancellationToken cancellationToken = default) =>
            Task.FromResult(_seen.Add((processedEvent.Provider, processedEvent.EventId)) ? ProcessedEventDecision.FirstDelivery : ProcessedEventDecision.Duplicate);
    }
}
