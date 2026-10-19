using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;

namespace Platform.Webhooks.Tests.Inbound;

public sealed class InMemoryWebhookInboxStoreTests
{
    [Fact]
    public async Task First_claim_records_message()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_1", clock.UtcNow);

        var result = await store.TryClaimAsync(message, clock, "worker-1", TimeSpan.FromMinutes(1));

        Assert.Equal(WebhookInboxClaimStatus.Claimed, result.Status);
        Assert.Equal(WebhookInboxState.Leased, result.Message!.State);
        Assert.Equal(1, store.Count);
    }

    [Fact]
    public async Task Duplicate_after_completion_is_detected()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_1", clock.UtcNow);
        var first = await store.TryClaimAsync(message, clock, "worker-1", TimeSpan.FromMinutes(1));
        await store.MarkCompletedAsync(first.Message!.ReplayKey, "worker-1");

        var retry = await store.TryClaimAsync(message, clock, "worker-2", TimeSpan.FromMinutes(1));

        Assert.Equal(WebhookInboxClaimStatus.Duplicate, retry.Status);
    }

    [Fact]
    public async Task Concurrent_lease_is_busy()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_1", clock.UtcNow);
        var first = await store.TryClaimAsync(message, clock, "worker-1", TimeSpan.FromMinutes(5));

        var busy = await store.TryClaimAsync(message, clock, "worker-2", TimeSpan.FromMinutes(5));

        Assert.Equal(WebhookInboxClaimStatus.Claimed, first.Status);
        Assert.Equal(WebhookInboxClaimStatus.Busy, busy.Status);
    }

    [Fact]
    public async Task Failure_is_recorded_with_lease_owner()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_1", clock.UtcNow);
        var first = await store.TryClaimAsync(message, clock, "worker-1", TimeSpan.FromMinutes(1));

        await store.MarkFailedAsync(first.Message!.ReplayKey, "worker-1", new WebhookFailure("webhook.handler_transient", "Transient.", true), clock.UtcNow.AddMinutes(1), deadLettered: false);

        var stored = await store.GetAsync(first.Message.ReplayKey);
        Assert.Equal(WebhookInboxState.Pending, stored!.State);
        Assert.Equal("webhook.handler_transient", stored.LastFailure!.Code);
    }

    [Fact]
    public async Task Different_lease_owner_cannot_complete()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var store = new InMemoryWebhookInboxStore();
        var message = WebhookInboxMessage.Create(new WebhookProviderId("test"), "evt_1", clock.UtcNow);
        await store.TryClaimAsync(message, clock, "worker-1", TimeSpan.FromMinutes(1));

        await Assert.ThrowsAsync<InvalidOperationException>(() => store.MarkCompletedAsync(message.ReplayKey, "worker-2"));
    }
}
