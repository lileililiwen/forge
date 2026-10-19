using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.Contracts.Tests;

public sealed class DurableEventStoreTests
{
    private static readonly DateTimeOffset Now = new(2026, 9, 8, 0, 0, 0, TimeSpan.Zero);

    [Fact]
    public async Task Outbox_claims_pending_message_and_completes_it()
    {
        var store = new InMemoryOutboxStore(new FixedClock(Now));
        var message = OutboxMessage.Create(
            new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now, "tenant-1", "corr-1"), Now);

        await store.AddAsync(message);
        var claimed = await store.ClaimAsync(Now, "worker-1", TimeSpan.FromMinutes(5), 10);

        var item = Assert.Single(claimed);
        Assert.Equal(DurableMessageState.Leased, item.State);
        Assert.Equal("worker-1", item.LeaseOwnerId);
        Assert.Equal(1, item.AttemptCount);

        await store.MarkSucceededAsync(item.MessageId, "worker-1");

        var remaining = await store.ClaimAsync(Now, "worker-2", TimeSpan.FromMinutes(5), 10);
        Assert.Empty(remaining);
    }

    [Fact]
    public async Task Outbox_releases_expired_lease_for_another_worker()
    {
        var clock = new MutableClock(Now);
        var store = new InMemoryOutboxStore(clock);
        await store.AddAsync(OutboxMessage.Create(
            new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now));

        await store.ClaimAsync(Now, "worker-1", TimeSpan.FromMinutes(5), 10);
        clock.Advance(TimeSpan.FromMinutes(6));

        var reclaimed = await store.ClaimAsync(clock.UtcNow, "worker-2", TimeSpan.FromMinutes(5), 10);
        Assert.Equal("worker-2", Assert.Single(reclaimed).LeaseOwnerId);
    }

    [Fact]
    public async Task Outbox_failed_message_becomes_dead_letter_after_attempt_limit()
    {
        var clock = new MutableClock(Now);
        var store = new InMemoryOutboxStore(clock, new DurableEventingOptions { MaxAttempts = 1 });
        await store.AddAsync(OutboxMessage.Create(
            new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now));

        var claimed = Assert.Single(await store.ClaimAsync(Now, "worker-1", TimeSpan.FromMinutes(5), 10));
        await store.MarkFailedAsync(claimed.MessageId, "worker-1", new DurableDispatchFailure("permanent", "invalid payload"));

        Assert.Equal(DurableMessageState.DeadLetter, (await store.GetAsync(claimed.MessageId))!.State);
    }

    [Fact]
    public async Task Inbox_returns_duplicate_for_completed_message()
    {
        var clock = new FixedClock(Now);
        var store = new InMemoryInboxStore(clock);
        var message = InboxMessage.Create(
            new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now);

        var first = await store.TryClaimAsync(message, Now, "worker-1", TimeSpan.FromMinutes(5));
        Assert.Equal(InboxClaimDecision.Claimed, first.Decision);
        await store.MarkSucceededAsync(message.MessageId, "worker-1");

        var duplicate = await store.TryClaimAsync(message, Now, "worker-2", TimeSpan.FromMinutes(5));
        Assert.Equal(InboxClaimDecision.Duplicate, duplicate.Decision);
    }

    [Fact]
    public async Task Concurrent_outbox_claims_assign_a_message_to_only_one_worker()
    {
        var store = new InMemoryOutboxStore(new FixedClock(Now));
        await store.AddAsync(OutboxMessage.Create(
            new DurableEventEnvelope("message-1", "Orders.OrderPlaced", "{}", Now), Now));

        var claims = await Task.WhenAll(Enumerable.Range(1, 8).Select(worker =>
            store.ClaimAsync(Now, $"worker-{worker}", TimeSpan.FromMinutes(5), 10)));

        Assert.Single(claims.SelectMany(result => result));
    }

    [Fact]
    public void Envelope_rejects_invalid_required_values()
    {
        Assert.Throws<ArgumentException>(() => new DurableEventEnvelope("", "type", "{}", Now));
        Assert.Throws<ArgumentException>(() => new DurableEventEnvelope("id", "", "{}", Now));
        Assert.Throws<ArgumentException>(() => new DurableEventEnvelope("id", "type", "", Now));
        Assert.Throws<ArgumentException>(() => new DurableEventEnvelope("id", "type", "{}", Now, correlationId: "bad\nvalue"));
    }
}

internal sealed class MutableClock(DateTimeOffset initial) : IClock
{
    public DateTimeOffset UtcNow { get; private set; } = initial;

    public void Advance(TimeSpan amount) => UtcNow = UtcNow.Add(amount);
}
