using Platform.Billing.Contracts.Events;
using Platform.Billing.Contracts.Identifiers;

namespace Platform.Billing.Contracts.Tests.Events;

public class ProcessedEventStoreContractTests
{
    [Fact]
    public async Task First_delivery_is_recorded_and_returns_first_delivery()
    {
        IProcessedEventStore store = new InMemoryProcessedEventStore();
        var evt = NewEvent("evt_1");

        var decision = await store.MarkProcessedAsync(NewProcessedEvent(evt, "applied"));

        Assert.Equal(ProcessedEventDecision.FirstDelivery, decision);
    }

    [Fact]
    public async Task Redelivered_event_returns_duplicate()
    {
        IProcessedEventStore store = new InMemoryProcessedEventStore();
        var evt = NewEvent("evt_1");
        await store.MarkProcessedAsync(NewProcessedEvent(evt, "applied"));

        var decision = await store.MarkProcessedAsync(NewProcessedEvent(evt, "applied"));

        Assert.Equal(ProcessedEventDecision.Duplicate, decision);
    }

    [Fact]
    public async Task Duplicate_does_not_overwrite_original_record()
    {
        IProcessedEventStore store = new InMemoryProcessedEventStore();
        var evt = NewEvent("evt_1");
        var firstTime = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        await store.MarkProcessedAsync(new ProcessedEvent(evt.Id, evt.Provider, firstTime, "applied"));

        var laterTime = firstTime.AddHours(1);
        var decision = await store.MarkProcessedAsync(new ProcessedEvent(evt.Id, evt.Provider, laterTime, "ignored"));

        Assert.Equal(ProcessedEventDecision.Duplicate, decision);
    }

    [Fact]
    public async Task Different_providers_with_same_event_id_are_independent()
    {
        IProcessedEventStore store = new InMemoryProcessedEventStore();
        var stripeEvent = NewEvent("evt_1", "stripe");
        var paddleEvent = NewEvent("evt_1", "paddle");

        var firstDecision = await store.MarkProcessedAsync(NewProcessedEvent(stripeEvent, "applied"));
        var secondDecision = await store.MarkProcessedAsync(NewProcessedEvent(paddleEvent, "applied"));

        Assert.Equal(ProcessedEventDecision.FirstDelivery, firstDecision);
        Assert.Equal(ProcessedEventDecision.FirstDelivery, secondDecision);
    }

    private static ProviderEvent NewEvent(string id, string provider = "stripe") =>
        new(
            ProviderEventId.Create(id),
            ProviderName.Create(provider),
            "subscription.updated",
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            "{}");

    private static ProcessedEvent NewProcessedEvent(ProviderEvent evt, string result) =>
        new(evt.Id, evt.Provider, evt.OccurredAt, result);

    private sealed class InMemoryProcessedEventStore : IProcessedEventStore
    {
        private readonly Dictionary<(ProviderEventId, ProviderName), ProcessedEvent> _store = new();

        public Task<ProcessedEventDecision> MarkProcessedAsync(ProcessedEvent processedEvent, CancellationToken cancellationToken = default)
        {
            var key = (processedEvent.EventId, processedEvent.Provider);
            if (_store.ContainsKey(key))
            {
                return Task.FromResult(ProcessedEventDecision.Duplicate);
            }
            _store[key] = processedEvent;
            return Task.FromResult(ProcessedEventDecision.FirstDelivery);
        }
    }
}
