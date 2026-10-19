using Platform.Billing.Contracts.Entitlements;
using Platform.Billing.Contracts.Features;
using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Tests.Features;

public class FeatureCheckTests
{
    [Fact]
    public void Anonymous_subject_is_denied_with_not_authenticated()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = EntitlementDefaults.Anonymous(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.False(result.IsAllowed);
        Assert.Equal(FeatureCheckReason.NotAuthenticated, result.Reason);
        Assert.Equal(feature, result.Feature);
    }

    [Fact]
    public void Inactive_subscription_is_denied_with_not_subscribed()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = NewSnapshot(SubscriptionStatus.Canceled, features: new HashSet<FeatureKey> { feature });

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.False(result.IsAllowed);
        Assert.Equal(FeatureCheckReason.NotSubscribed, result.Reason);
    }

    [Fact]
    public void Active_subscription_with_feature_is_allowed()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = NewSnapshot(SubscriptionStatus.Active, features: new HashSet<FeatureKey> { feature });

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.True(result.IsAllowed);
        Assert.Equal(FeatureCheckReason.Allowed, result.Reason);
    }

    [Fact]
    public void Active_subscription_without_feature_is_denied_with_plan_mismatch()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = NewSnapshot(SubscriptionStatus.Active, features: new HashSet<FeatureKey>());

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.False(result.IsAllowed);
        Assert.Equal(FeatureCheckReason.PlanMismatch, result.Reason);
    }

    [Fact]
    public void Free_subscription_is_treated_as_active()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = NewSnapshot(SubscriptionStatus.Free, features: new HashSet<FeatureKey> { feature });

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.True(result.IsAllowed);
    }

    [Fact]
    public void Unknown_subscription_state_is_denied()
    {
        var feature = FeatureKey.Create("export.csv");
        var snapshot = NewSnapshot(SubscriptionStatus.Unknown, features: new HashSet<FeatureKey> { feature });

        var result = FeatureCheck.Evaluate(snapshot, feature);

        Assert.False(result.IsAllowed);
        Assert.Equal(FeatureCheckReason.NotSubscribed, result.Reason);
    }

    [Fact]
    public void Denied_carries_required_plan_when_supplied()
    {
        var feature = FeatureKey.Create("export.csv");
        var required = PlanId.Create("plan.pro");

        var result = FeatureCheck.Denied(feature, FeatureCheckReason.PlanMismatch, required);

        Assert.Equal(required, result.RequiredPlan);
    }

    [Fact]
    public void Evaluate_rejects_null_entitlement()
    {
        Assert.Throws<ArgumentNullException>(() =>
            FeatureCheck.Evaluate(null!, FeatureKey.Create("x")));
    }

    private static Entitlement NewSnapshot(SubscriptionStatus status, IReadOnlySet<FeatureKey> features)
    {
        var now = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var start = now;
        var end = start.AddDays(30);
        var subscription = new Subscription(
            SubjectKey.Create("user-1"),
            PlanId.Create("plan.pro"),
            status,
            ProviderName.Create("stripe"),
            "sub_test",
            start,
            end);
        return new Entitlement(
            SubjectKey.Create("user-1"),
            Tenant: null,
            subscription,
            features,
            Limits: null,
            now);
    }
}
