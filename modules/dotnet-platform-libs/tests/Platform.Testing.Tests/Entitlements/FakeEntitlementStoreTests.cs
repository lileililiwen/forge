using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Features;
using Platform.Billing.Contracts.Identifiers;
using Platform.Testing.Entitlements;
using Platform.Testing.Time;

namespace Platform.Testing.Tests.Entitlements;

public class FakeEntitlementStoreTests
{
    [Fact]
    public void Unknown_subject_returns_inactive_default()
    {
        var store = new FakeEntitlementStore();
        var subject = SubjectKey.Create("user-1");
        var captured = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);

        var snapshot = store.Get(subject, captured);

        Assert.Equal(subject, snapshot.Subject);
        Assert.Null(snapshot.Subscription);
        Assert.Empty(snapshot.ActiveFeatures);
    }

    [Fact]
    public void Configured_snapshot_is_returned_on_lookup()
    {
        var store = new FakeEntitlementStore();
        var clock = new ControllableClock();
        var subject = SubjectKey.Create("user-1");
        var subscription = SubscriptionBuilder.For(subject).Build(clock);
        var configured = EntitlementBuilder.For(subject)
            .WithSubscription(subscription)
            .Granting(FeatureKey.Create("export.csv"))
            .Build(clock);

        store.Configure(subject, configured);

        var snapshot = store.Get(subject, clock.UtcNow);
        Assert.Same(configured, snapshot);
        Assert.True(snapshot.Grants(FeatureKey.Create("export.csv")));
    }

    [Fact]
    public void Invalidate_returns_inactive_default_until_reconfigured()
    {
        var store = new FakeEntitlementStore();
        var clock = new ControllableClock();
        var subject = SubjectKey.Create("user-1");
        store.Configure(subject, EntitlementBuilder.For(subject)
            .Granting(FeatureKey.Create("export.csv"))
            .Build(clock));

        store.Invalidate(subject);
        var snapshot = store.Get(subject, clock.UtcNow);

        Assert.Empty(snapshot.ActiveFeatures);
        Assert.Null(snapshot.Subscription);
    }

    [Fact]
    public void Configure_after_invalidation_replaces_default()
    {
        var store = new FakeEntitlementStore();
        var clock = new ControllableClock();
        var subject = SubjectKey.Create("user-1");
        var feature = FeatureKey.Create("export.csv");
        store.Invalidate(subject);

        var newSnapshot = EntitlementBuilder.For(subject)
            .Granting(feature)
            .Build(clock);
        store.Configure(subject, newSnapshot);

        var snapshot = store.Get(subject, clock.UtcNow);
        Assert.True(snapshot.Grants(feature));
    }

    [Fact]
    public void InvalidatedSubjects_exposes_invalidated_keys()
    {
        var store = new FakeEntitlementStore();
        var subject = SubjectKey.Create("user-1");

        store.Invalidate(subject);

        Assert.Contains(subject, store.InvalidatedSubjects);
    }

    [Fact]
    public void Configure_clears_invalidation()
    {
        var store = new FakeEntitlementStore();
        var clock = new ControllableClock();
        var subject = SubjectKey.Create("user-1");
        store.Invalidate(subject);

        store.Configure(subject, EntitlementBuilder.For(subject).Build(clock));

        Assert.DoesNotContain(subject, store.InvalidatedSubjects);
    }

    [Fact]
    public void Reset_clears_configured_and_invalidated_state()
    {
        var store = new FakeEntitlementStore();
        var clock = new ControllableClock();
        var subject = SubjectKey.Create("user-1");
        store.Configure(subject, EntitlementBuilder.For(subject).Build(clock));
        store.Invalidate(SubjectKey.Create("user-2"));

        store.Reset();

        Assert.Empty(store.InvalidatedSubjects);
        Assert.Empty(store.Get(subject, clock.UtcNow).ActiveFeatures);
    }

    [Fact]
    public void Configure_rejects_null_entitlement()
    {
        var store = new FakeEntitlementStore();
        Assert.Throws<ArgumentNullException>(() => store.Configure(SubjectKey.Create("user-1"), null!));
    }
}
