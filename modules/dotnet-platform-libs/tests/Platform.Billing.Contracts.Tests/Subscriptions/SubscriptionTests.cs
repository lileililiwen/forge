using Platform.Billing.Contracts.Identifiers;
using Platform.Billing.Contracts.Subscriptions;

namespace Platform.Billing.Contracts.Tests.Subscriptions;

public class SubscriptionTests
{
    [Fact]
    public void Active_status_is_active()
    {
        var subscription = NewSubscription(SubscriptionStatus.Active);
        Assert.True(subscription.IsActive);
    }

    [Fact]
    public void Free_status_is_active()
    {
        var subscription = NewSubscription(SubscriptionStatus.Free);
        Assert.True(subscription.IsActive);
    }

    [Theory]
    [InlineData(SubscriptionStatus.Suspended)]
    [InlineData(SubscriptionStatus.PastDue)]
    [InlineData(SubscriptionStatus.Canceled)]
    [InlineData(SubscriptionStatus.Unknown)]
    public void Non_free_non_active_statuses_are_inactive(SubscriptionStatus status)
    {
        var subscription = NewSubscription(status);
        Assert.False(subscription.IsActive);
    }

    [Fact]
    public void IsWithinPeriod_inclusive_of_start_exclusive_of_end()
    {
        var start = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var end = start.AddDays(30);
        var subscription = NewSubscription(SubscriptionStatus.Active, start, end);

        Assert.True(subscription.IsWithinPeriod(start));
        Assert.True(subscription.IsWithinPeriod(start.AddSeconds(1)));
        Assert.False(subscription.IsWithinPeriod(end));
        Assert.False(subscription.IsWithinPeriod(end.AddSeconds(1)));
    }

    private static Subscription NewSubscription(SubscriptionStatus status, DateTimeOffset? start = null, DateTimeOffset? end = null)
    {
        var periodStart = start ?? new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var periodEnd = end ?? periodStart.AddDays(30);
        return new Subscription(
            SubjectKey.Create("user-1"),
            PlanId.Create("plan.pro"),
            status,
            ProviderName.Create("stripe"),
            "sub_test",
            periodStart,
            periodEnd);
    }
}
