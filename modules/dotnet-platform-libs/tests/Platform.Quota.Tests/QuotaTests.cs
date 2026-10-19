using Platform.Core.Time;
using Platform.Quota.Contracts;
using Platform.Quota.Stores;
using Platform.Quota.Testing;

namespace Platform.Quota.Tests;

public sealed class QuotaTests
{
    [Fact]
    public async Task Decision_explains_capacity_and_subject_resource_window()
    {
        var scenario = new QuotaScenarioBuilder().WithLimit(10).WithConsumed(3).Build();
        var store = new InMemoryQuotaStore(scenario.Clock, initialConsumed: scenario.Consumed);

        var decision = await store.CheckAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, 4);

        Assert.True(decision.Allowed);
        Assert.Equal(4, decision.Requested);
        Assert.Equal(3, decision.Consumed);
        Assert.Equal(10, decision.Limit);
        Assert.Equal(7, decision.Remaining);
        Assert.Equal(scenario.Subject, decision.Subject);
        Assert.Equal(scenario.Resource, decision.Resource);
        Assert.Equal(scenario.Window, decision.Window);
    }

    [Fact]
    public async Task Reservation_is_idempotent_and_release_is_repeatable()
    {
        var scenario = new QuotaScenarioBuilder().WithLimit(10).Build();
        var store = new InMemoryQuotaStore(scenario.Clock);

        var first = await store.ReserveAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, "operation-1", 6);
        var retry = await store.ReserveAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, "operation-1", 6);
        var release = await store.ReleaseAsync("operation-1");
        var repeated = await store.ReleaseAsync("operation-1");
        var next = await store.CheckAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, 10);

        Assert.True(first.Decision!.Allowed);
        Assert.Equal(first.Decision, retry.Decision);
        Assert.Equal(QuotaLifecycleStatus.Released, release.Status);
        Assert.Equal(QuotaLifecycleStatus.AlreadyReleased, repeated.Status);
        Assert.True(next.Allowed);
    }

    [Fact]
    public async Task Settlement_is_idempotent_and_invalid_transition_is_safe()
    {
        var scenario = new QuotaScenarioBuilder().WithLimit(10).Build();
        var store = new InMemoryQuotaStore(scenario.Clock);
        await store.ReserveAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, "operation-2", 4);

        var settled = await store.SettleAsync("operation-2");
        var retry = await store.SettleAsync("operation-2");
        var invalid = await store.ReleaseAsync("missing");

        Assert.Equal(QuotaLifecycleStatus.Settled, settled.Status);
        Assert.Equal(QuotaLifecycleStatus.AlreadySettled, retry.Status);
        Assert.Equal(QuotaLifecycleStatus.NotFound, invalid.Status);
    }

    [Fact]
    public async Task Concurrent_reservations_cannot_oversubscribe_limit()
    {
        var scenario = new QuotaScenarioBuilder().WithLimit(1).Build();
        var store = new InMemoryQuotaStore(scenario.Clock);
        var results = await Task.WhenAll(Enumerable.Range(0, 32).Select(index =>
            store.ReserveAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit, $"operation-{index}", 1)));

        Assert.Single(results, result => result.Decision?.Allowed == true);
        Assert.Equal(1, (await store.GetSnapshotAsync(scenario.Subject, scenario.Resource, scenario.Window, scenario.Limit)).Reserved);
    }

    [Fact]
    public void Options_and_identifiers_reject_invalid_values()
    {
        Assert.Throws<ArgumentException>(() => new QuotaSubject(" "));
        Assert.Throws<ArgumentException>(() => new QuotaResource("resource with spaces"));
        Assert.Throws<ArgumentException>(() => new QuotaOperationKey(" "));
        Assert.Throws<ArgumentOutOfRangeException>(() => new QuotaOptions { MaximumAmount = 0 }.Validate());
        var now = DateTimeOffset.UtcNow;
        Assert.Throws<ArgumentOutOfRangeException>(() => new QuotaWindow(now, now).Validate());
    }
}
