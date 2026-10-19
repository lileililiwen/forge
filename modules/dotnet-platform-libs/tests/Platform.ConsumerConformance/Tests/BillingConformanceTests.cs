using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Billing;
using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Billing.Contracts.Entitlements;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Testing.Entitlements;

namespace Platform.ConsumerConformance.Tests;

public sealed class BillingConformanceTests
{
    [Fact]
    public async Task BillingEventOrchestrator_deduplicates_provider_events()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryProcessedEventStore();
        var projector = new InMemoryBillingProjector();
        var orchestrator = new BillingEventOrchestrator(store, projector, clock);

        var first = new ProviderEvent(
            ProviderEventId.Create("evt-1"),
            ProviderName.Create("stripe"),
            "subscription.activated",
            clock.UtcNow,
            "{\"subject\":\"u1\"}");
        var second = new ProviderEvent(
            ProviderEventId.Create("evt-1"),
            ProviderName.Create("stripe"),
            "subscription.activated",
            clock.UtcNow,
            "{\"subject\":\"u1\"}");

        var firstDecision = await orchestrator.ProcessAsync(new ProviderEventEnvelope(first), CancellationToken.None);
        var secondDecision = await orchestrator.ProcessAsync(new ProviderEventEnvelope(second), CancellationToken.None);

        Assert.Equal(BillingEventDecision.Applied, firstDecision);
        Assert.Equal(BillingEventDecision.Duplicate, secondDecision);
    }

    [Fact]
    public async Task BillingEventOrchestrator_rejects_stale_events()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryProcessedEventStore();
        var projector = new InMemoryBillingProjector();
        var orchestrator = new BillingEventOrchestrator(store, projector, clock);

        var earlier = new ProviderEvent(
            ProviderEventId.Create("evt-1"),
            ProviderName.Create("stripe"),
            "subscription.activated",
            clock.UtcNow,
            "{\"subject\":\"u1\"}");
        var later = new ProviderEvent(
            ProviderEventId.Create("evt-2"),
            ProviderName.Create("stripe"),
            "subscription.activated",
            clock.UtcNow.AddMinutes(-1),
            "{\"subject\":\"u1\"}");

        var firstDecision = await orchestrator.ProcessAsync(new ProviderEventEnvelope(earlier), CancellationToken.None);
        var secondDecision = await orchestrator.ProcessAsync(new ProviderEventEnvelope(later), CancellationToken.None);

        Assert.Equal(BillingEventDecision.Applied, firstDecision);
        Assert.Equal(BillingEventDecision.IgnoredStale, secondDecision);
    }

    [Fact]
    public void Billing_testing_helpers_build_subscriptions_and_entitlements()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscription = SubscriptionBuilder.For(SubjectKey.Create("u1"))
            .WithPlan(PlanId.Create("plan-pro"))
            .WithStatus(SubscriptionStatus.Active)
            .Build(clock);
        Assert.Equal(SubscriptionStatus.Active, subscription.Status);
        Assert.Equal(PlanId.Create("plan-pro"), subscription.Plan);

        var entitlement = EntitlementBuilder.For(SubjectKey.Create("u1"))
            .WithSubscription(subscription)
            .Granting(FeatureKey.Create("feature-a"))
            .Build(clock);
        Assert.True(entitlement.Grants(FeatureKey.Create("feature-a")));
        Assert.False(entitlement.Grants(FeatureKey.Create("feature-b")));
    }

    private sealed class InMemoryProcessedEventStore : IProcessedEventStore
    {
        private readonly Dictionary<string, ProcessedEvent> _events = [];

        public Task<ProcessedEventDecision> MarkProcessedAsync(ProcessedEvent processedEvent, CancellationToken cancellationToken = default)
        {
            if (_events.ContainsKey(processedEvent.EventId.Value))
            {
                return Task.FromResult(ProcessedEventDecision.Duplicate);
            }
            _events[processedEvent.EventId.Value] = processedEvent;
            return Task.FromResult(ProcessedEventDecision.FirstDelivery);
        }
    }
}
