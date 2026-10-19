using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;
using Platform.Testing.Entitlements;
using Platform.Testing.Time;

namespace Platform.Testing.Tests.Entitlements;

public class SubscriptionBuilderTests
{
    [Fact]
    public void Build_anchors_period_at_clock_when_not_set()
    {
        var clock = new ControllableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subject = SubjectKey.Create("user-1");

        var subscription = SubscriptionBuilder.For(subject).Build(clock);

        Assert.Equal(subject, subscription.Subject);
        Assert.Equal(SubscriptionStatus.Active, subscription.Status);
        Assert.Equal(clock.UtcNow, subscription.PeriodStart);
        Assert.Equal(clock.UtcNow.AddDays(30), subscription.PeriodEnd);
        Assert.True(subscription.IsActive);
    }

    [Fact]
    public void WithPlan_overrides_default_plan()
    {
        var clock = new ControllableClock();
        var plan = PlanId.Create("plan.enterprise");

        var subscription = SubscriptionBuilder.For(SubjectKey.Create("user-1"))
            .WithPlan(plan)
            .Build(clock);

        Assert.Equal(plan, subscription.Plan);
    }

    [Fact]
    public void WithStatus_overrides_default_status()
    {
        var clock = new ControllableClock();

        var subscription = SubscriptionBuilder.For(SubjectKey.Create("user-1"))
            .WithStatus(SubscriptionStatus.Canceled)
            .Build(clock);

        Assert.Equal(SubscriptionStatus.Canceled, subscription.Status);
        Assert.False(subscription.IsActive);
    }

    [Fact]
    public void WithPeriodStart_and_WithPeriodEnd_overrides_defaults()
    {
        var clock = new ControllableClock();
        var start = new DateTimeOffset(2025, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var end = new DateTimeOffset(2025, 12, 31, 0, 0, 0, TimeSpan.Zero);

        var subscription = SubscriptionBuilder.For(SubjectKey.Create("user-1"))
            .WithPeriodStart(start)
            .WithPeriodEnd(end)
            .Build(clock);

        Assert.Equal(start, subscription.PeriodStart);
        Assert.Equal(end, subscription.PeriodEnd);
    }

    [Fact]
    public void Build_rejects_null_clock()
    {
        Assert.Throws<ArgumentNullException>(() =>
            SubscriptionBuilder.For(SubjectKey.Create("user-1")).Build(null!));
    }
}
