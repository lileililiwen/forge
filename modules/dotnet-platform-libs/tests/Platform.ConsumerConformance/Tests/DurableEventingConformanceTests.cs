using Microsoft.Extensions.DependencyInjection;
using Platform.ConsumerConformance.Fixtures;
using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.ConsumerConformance.Tests;

public sealed class DurableEventingConformanceTests
{
    [Fact]
    public async Task In_memory_outbox_store_round_trips_messages()
    {
        var store = new InMemoryOutboxStore(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        var message = OutboxMessage.Create(
            new DurableEventEnvelope(
                "msg-1",
                "Test.Event",
                "{\"Text\":\"hi\"}",
                new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)),
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        await store.AddAsync(message, CancellationToken.None);
        var claimed = await store.ClaimAsync(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), "worker-1", TimeSpan.FromSeconds(30), 10, CancellationToken.None);
        var first = Assert.Single(claimed);
        Assert.Equal(message.MessageId, first.MessageId);
    }

    [Fact]
    public async Task In_memory_inbox_store_detects_duplicates()
    {
        var store = new InMemoryInboxStore(new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)));
        var message = InboxMessage.Create(
            new DurableEventEnvelope(
                "msg-1",
                "Test.Event",
                "{\"Text\":\"hi\"}",
                new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)),
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        var first = await store.TryClaimAsync(message, new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), "worker-1", TimeSpan.FromSeconds(30), CancellationToken.None);
        await store.MarkSucceededAsync(message.MessageId, "worker-1", CancellationToken.None);
        var second = await store.TryClaimAsync(message, new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero), "worker-2", TimeSpan.FromSeconds(30), CancellationToken.None);
        Assert.Equal(InboxClaimDecision.Claimed, first.Decision);
        Assert.Equal(InboxClaimDecision.Duplicate, second.Decision);
    }
}
