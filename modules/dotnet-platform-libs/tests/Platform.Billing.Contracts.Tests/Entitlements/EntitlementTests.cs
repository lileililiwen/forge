using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Tests.Entitlements;

public class EntitlementTests
{
    [Fact]
    public void Anonymous_default_is_not_active()
    {
        var snapshot = EntitlementDefaults.Anonymous(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Equal(SubjectKey.Anonymous, snapshot.Subject);
        Assert.Null(snapshot.Tenant);
        Assert.Null(snapshot.Subscription);
        Assert.Empty(snapshot.ActiveFeatures);
    }

    [Fact]
    public void Inactive_for_known_subject_with_no_subscription()
    {
        var subject = SubjectKey.Create("user-1");
        var snapshot = EntitlementDefaults.Inactive(subject, new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Equal(subject, snapshot.Subject);
        Assert.Null(snapshot.Subscription);
    }

    [Fact]
    public void Unknown_subscription_is_preserved_but_grants_no_features()
    {
        var subject = SubjectKey.Create("user-1");
        var subscription = NewSubscription(status: SubscriptionStatus.Unknown);
        var snapshot = EntitlementDefaults.Unknown(subject, subscription, new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Same(subscription, snapshot.Subscription);
        Assert.NotNull(snapshot.Subscription);
        Assert.False(snapshot.Subscription!.IsActive);
        Assert.Empty(snapshot.ActiveFeatures);
    }

    [Fact]
    public void Grants_returns_true_for_active_feature()
    {
        var feature = FeatureKey.Create("export.csv");
        var features = new HashSet<FeatureKey> { feature };
        var snapshot = NewSnapshot(features: features);

        Assert.True(snapshot.Grants(feature));
    }

    [Fact]
    public void LimitFor_returns_configured_limit()
    {
        var feature = FeatureKey.Create("api.calls");
        var limits = new Dictionary<FeatureKey, long>
        {
            [feature] = 1000,
        };
        var snapshot = NewSnapshot(limits: limits);

        Assert.Equal(1000L, snapshot.LimitFor(feature));
    }

    [Fact]
    public void LimitFor_returns_null_when_unconfigured()
    {
        var snapshot = NewSnapshot();

        Assert.Null(snapshot.LimitFor(FeatureKey.Create("api.calls")));
    }

    private static Entitlement NewSnapshot(
        IReadOnlySet<FeatureKey>? features = null,
        IReadOnlyDictionary<FeatureKey, long>? limits = null)
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        return new Entitlement(
            SubjectKey.Create("user-1"),
            Tenant: null,
            NewSubscription(status: SubscriptionStatus.Active),
            features ?? new HashSet<FeatureKey>(),
            limits,
            now);
    }

    private static Subscription NewSubscription(SubscriptionStatus status)
    {
        var start = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var end = start.AddDays(30);
        return new Subscription(
            SubjectKey.Create("user-1"),
            PlanId.Create("plan.pro"),
            status,
            ProviderName.Create("stripe"),
            "sub_test",
            start,
            end);
    }
}
