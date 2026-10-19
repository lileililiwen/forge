using Platform.Billing.Contracts.Identifiers;
using Platform.Testing.Entitlements;
using Platform.Testing.Time;

namespace Platform.Testing.Tests.Entitlements;

public class EntitlementBuilderTests
{
    [Fact]
    public void Build_with_no_modifications_yields_inactive_snapshot()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subject = SubjectKey.Create("user-1");

        var snapshot = EntitlementBuilder.For(subject).Build(clock);

        Assert.Equal(subject, snapshot.Subject);
        Assert.Null(snapshot.Tenant);
        Assert.Null(snapshot.Subscription);
        Assert.Empty(snapshot.ActiveFeatures);
        Assert.Null(snapshot.Limits);
        Assert.Equal(clock.UtcNow, snapshot.CapturedAt);
    }

    [Fact]
    public void Granting_adds_single_feature()
    {
        var clock = new ControllableClock();
        var feature = FeatureKey.Create("export.csv");

        var snapshot = EntitlementBuilder.For(SubjectKey.Create("user-1"))
            .Granting(feature)
            .Build(clock);

        Assert.True(snapshot.Grants(feature));
    }

    [Fact]
    public void Granting_adds_multiple_features()
    {
        var clock = new ControllableClock();
        var features = new[]
        {
            FeatureKey.Create("export.csv"),
            FeatureKey.Create("api.access"),
        };

        var snapshot = EntitlementBuilder.For(SubjectKey.Create("user-1"))
            .Granting(features)
            .Build(clock);

        Assert.Equal(2, snapshot.ActiveFeatures.Count);
    }

    [Fact]
    public void WithLimit_records_limit_for_feature()
    {
        var clock = new ControllableClock();
        var feature = FeatureKey.Create("api.calls");

        var snapshot = EntitlementBuilder.For(SubjectKey.Create("user-1"))
            .WithLimit(feature, 1000)
            .Build(clock);

        Assert.Equal(1000L, snapshot.LimitFor(feature));
    }

    [Fact]
    public void CapturedAt_overrides_clock()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var captured = new DateTimeOffset(2025, 6, 1, 0, 0, 0, TimeSpan.Zero);

        var snapshot = EntitlementBuilder.For(SubjectKey.Create("user-1"))
            .CapturedAt(captured)
            .Build(clock);

        Assert.Equal(captured, snapshot.CapturedAt);
    }

    [Fact]
    public void Build_rejects_null_clock()
    {
        Assert.Throws<ArgumentNullException>(() =>
            EntitlementBuilder.For(SubjectKey.Create("user-1")).Build(null!));
    }
}
