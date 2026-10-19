using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Usage;

namespace Platform.Billing.Contracts.Tests.Usage;

public class UsageMeterContractTests
{
    [Fact]
    public async Task In_memory_meter_returns_recorded_usage()
    {
        IUsageMeter meter = new InMemoryUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        await meter.RecordAsync(subject, feature, 3);
        var result = await meter.CheckAsync(subject, feature);

        Assert.Equal(feature, result.Feature);
        Assert.Equal(3L, result.Used);
    }

    [Fact]
    public async Task Meter_returns_within_limit_when_below_configured_limit()
    {
        IUsageMeter meter = new InMemoryUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        await meter.RecordAsync(subject, feature, 5);

        var result = await meter.CheckAsync(subject, feature);

        Assert.True(result.IsWithinLimit);
    }

    [Fact]
    public async Task Meter_accumulates_across_record_calls()
    {
        IUsageMeter meter = new InMemoryUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        await meter.RecordAsync(subject, feature, 2);
        await meter.RecordAsync(subject, feature, 3);
        var result = await meter.CheckAsync(subject, feature);

        Assert.Equal(5L, result.Used);
    }

    [Fact]
    public async Task Meter_keeps_subjects_isolated()
    {
        IUsageMeter meter = new InMemoryUsageMeter();
        var feature = FeatureKey.Create("api.calls");
        var alice = SubjectKey.Create("alice");
        var bob = SubjectKey.Create("bob");

        await meter.RecordAsync(alice, feature, 2);
        await meter.RecordAsync(bob, feature, 7);

        var aliceResult = await meter.CheckAsync(alice, feature);
        var bobResult = await meter.CheckAsync(bob, feature);

        Assert.Equal(2L, aliceResult.Used);
        Assert.Equal(7L, bobResult.Used);
    }

    [Fact]
    public async Task Meter_is_within_limit_when_no_limit_is_configured()
    {
        IUsageMeter meter = new InMemoryUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        var result = await meter.CheckAsync(subject, feature);

        Assert.Null(result.Limit);
        Assert.True(result.IsWithinLimit);
    }

    [Fact]
    public async Task RecordAsync_rejects_negative_units()
    {
        IUsageMeter meter = new InMemoryUsageMeter();

        await Assert.ThrowsAsync<ArgumentOutOfRangeException>(() =>
            meter.RecordAsync(SubjectKey.Create("user-1"), FeatureKey.Create("x"), -1));
    }

    private sealed class InMemoryUsageMeter : IUsageMeter
    {
        private readonly Dictionary<(SubjectKey, FeatureKey), long> _counts = new();

        public Task<UsageCheckResult> CheckAsync(SubjectKey subject, FeatureKey feature, CancellationToken cancellationToken = default)
        {
            var used = _counts.TryGetValue((subject, feature), out var current) ? current : 0L;
            return Task.FromResult(new UsageCheckResult(feature, used, Limit: null, WindowStart: null, WindowEnd: null));
        }

        public Task<UsageCheckResult> RecordAsync(SubjectKey subject, FeatureKey feature, long units, CancellationToken cancellationToken = default)
        {
            if (units < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(units), units, "Units must be non-negative.");
            }
            var key = (subject, feature);
            _counts[key] = (_counts.TryGetValue(key, out var current) ? current : 0L) + units;
            return Task.FromResult(new UsageCheckResult(feature, _counts[key], Limit: null, WindowStart: null, WindowEnd: null));
        }
    }
}
