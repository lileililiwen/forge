using Microsoft.Extensions.Logging.Abstractions;
using Platform.Core.Time;
using Platform.Eventing.Contracts;
using Platform.Eventing.EfCore;

namespace Platform.Eventing.EfCore.Tests;

public sealed class DurableOutboxDispatcherTests
{
    private static readonly DateTimeOffset Now = new(2026, 9, 8, 0, 0, 0, TimeSpan.Zero);

    [Fact]
    public async Task Dispatch_batch_publishes_and_completes_claimed_messages()
    {
        var clock = new FixedClock(Now);
        var store = new InMemoryOutboxStore(clock);
        var publisher = new RecordingPublisher();
        await store.AddAsync(OutboxMessage.Create(new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now));
        await using var dispatcher = new DurableOutboxDispatcher(
            store,
            publisher,
            clock,
            new DurableEventingOptions { BatchSize = 1 },
            NullLogger<DurableOutboxDispatcher>.Instance,
            "worker-1");

        var published = await dispatcher.DispatchBatchAsync();

        Assert.Equal(1, published);
        Assert.Equal("message-1", Assert.Single(publisher.Messages).MessageId);
        Assert.Equal(DurableMessageState.Succeeded, (await store.GetAsync("message-1"))!.State);
    }

    [Fact]
    public async Task Dispatch_failure_records_safe_dead_letter_state()
    {
        var clock = new FixedClock(Now);
        var store = new InMemoryOutboxStore(clock, new DurableEventingOptions { MaxAttempts = 1 });
        await store.AddAsync(OutboxMessage.Create(new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now));
        await using var dispatcher = new DurableOutboxDispatcher(
            store,
            new ThrowingPublisher(),
            clock,
            new DurableEventingOptions { MaxAttempts = 1 },
            NullLogger<DurableOutboxDispatcher>.Instance,
            "worker-1");

        var published = await dispatcher.DispatchBatchAsync();

        Assert.Equal(0, published);
        var failed = await store.GetAsync("message-1");
        Assert.Equal(DurableMessageState.DeadLetter, failed!.State);
        Assert.Equal("durable_event.publish_failed", failed.LastFailure!.Code);
        Assert.DoesNotContain("secret", failed.LastFailure.Message, StringComparison.OrdinalIgnoreCase);
    }

    private sealed class RecordingPublisher : IDurableEventPublisher
    {
        public List<DurableEventEnvelope> Messages { get; } = [];
        public Task PublishAsync(DurableEventEnvelope envelope, CancellationToken cancellationToken = default)
        {
            Messages.Add(envelope);
            return Task.CompletedTask;
        }
    }

    private sealed class ThrowingPublisher : IDurableEventPublisher
    {
        public Task PublishAsync(DurableEventEnvelope envelope, CancellationToken cancellationToken = default) =>
            throw new InvalidOperationException("secret provider response");
    }
}
