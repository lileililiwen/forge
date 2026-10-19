using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Usage;
using Platform.Testing.Usage;

namespace Platform.Testing.Tests.Usage;

public class RecordingUsageMeterTests
{
    [Fact]
    public async Task Record_accumulates_units_per_subject_and_feature()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        await meter.RecordAsync(subject, feature, 3);
        await meter.RecordAsync(subject, feature, 4);
        var result = await meter.CheckAsync(subject, feature);

        Assert.Equal(7L, result.Used);
        Assert.Equal(7L, meter.TotalFor(subject, feature));
    }

    [Fact]
    public async Task Check_returns_zero_when_nothing_recorded()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        var result = await meter.CheckAsync(subject, feature);

        Assert.Equal(0L, result.Used);
    }

    [Fact]
    public async Task SetLimit_is_returned_by_check()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");
        meter.SetLimit(feature, 100);

        var result = await meter.CheckAsync(subject, feature);

        Assert.Equal(100L, result.Limit);
    }

    [Fact]
    public async Task Check_observes_exceeded_limit()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");
        meter.SetLimit(feature, 5);
        await meter.RecordAsync(subject, feature, 6);

        var result = await meter.CheckAsync(subject, feature);

        Assert.False(result.IsWithinLimit);
    }

    [Fact]
    public async Task Calls_records_check_and_record_operations()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");

        await meter.CheckAsync(subject, feature);
        await meter.RecordAsync(subject, feature, 2);
        await meter.CheckAsync(subject, feature);

        Assert.Equal(3, meter.Calls.Count);
        Assert.Equal(UsageCall.OperationKind.Check, meter.Calls[0].Operation);
        Assert.Equal(UsageCall.OperationKind.Record, meter.Calls[1].Operation);
        Assert.Equal(2L, meter.Calls[1].Units);
        Assert.Equal(UsageCall.OperationKind.Check, meter.Calls[2].Operation);
    }

    [Fact]
    public async Task Meter_keeps_subjects_isolated()
    {
        var meter = new RecordingUsageMeter();
        var feature = FeatureKey.Create("api.calls");
        var alice = SubjectKey.Create("alice");
        var bob = SubjectKey.Create("bob");

        await meter.RecordAsync(alice, feature, 3);
        await meter.RecordAsync(bob, feature, 7);

        Assert.Equal(3L, meter.TotalFor(alice, feature));
        Assert.Equal(7L, meter.TotalFor(bob, feature));
    }

    [Fact]
    public async Task RecordAsync_rejects_negative_units()
    {
        var meter = new RecordingUsageMeter();

        await Assert.ThrowsAsync<ArgumentOutOfRangeException>(() =>
            meter.RecordAsync(SubjectKey.Create("user-1"), FeatureKey.Create("x"), -1));
    }

    [Fact]
    public async Task Reset_clears_totals_and_calls_but_preserves_limits()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");
        meter.SetLimit(feature, 10);
        await meter.RecordAsync(subject, feature, 5);
        var beforeResetCalls = meter.Calls.Count;

        meter.Reset();

        Assert.Empty(meter.Calls);
        Assert.True(beforeResetCalls > 0);

        var result = await meter.CheckAsync(subject, feature);
        Assert.Equal(0L, result.Used);
        Assert.Equal(10L, result.Limit);
    }

    [Fact]
    public async Task SetLimit_with_null_removes_configured_limit()
    {
        var meter = new RecordingUsageMeter();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("api.calls");
        meter.SetLimit(feature, 10);

        meter.SetLimit(feature, null);

        var result = await meter.CheckAsync(subject, feature);
        Assert.Null(result.Limit);
    }
}
