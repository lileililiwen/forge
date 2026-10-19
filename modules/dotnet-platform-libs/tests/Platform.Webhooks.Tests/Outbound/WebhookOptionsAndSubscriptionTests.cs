using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Outbound;

namespace Platform.Webhooks.Tests.Outbound;

public sealed class WebhookOptionsAndSubscriptionTests
{
    [Fact]
    public void Options_reject_invalid_values()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new WebhookOptions { MaximumInboundBodyBytes = 0 }.Validate());
        Assert.Throws<ArgumentOutOfRangeException>(() => new WebhookOptions { DefaultOutboundTimeout = TimeSpan.Zero }.Validate());
        Assert.Throws<ArgumentOutOfRangeException>(() => new WebhookOptions { DefaultRetryBaseDelay = TimeSpan.FromMinutes(2), DefaultRetryMaxDelay = TimeSpan.FromMinutes(1) }.Validate());
    }

    [Fact]
    public void Retry_policy_computes_capped_exponential_backoff()
    {
        var policy = new WebhookRetryPolicy(5, TimeSpan.FromSeconds(1), TimeSpan.FromSeconds(8));
        Assert.Equal(TimeSpan.FromSeconds(1), policy.DelayFor(1));
        Assert.Equal(TimeSpan.FromSeconds(2), policy.DelayFor(2));
        Assert.Equal(TimeSpan.FromSeconds(4), policy.DelayFor(3));
        Assert.Equal(TimeSpan.FromSeconds(8), policy.DelayFor(4));
        Assert.Equal(TimeSpan.FromSeconds(8), policy.DelayFor(10));
    }

    [Fact]
    public void Identifiers_reject_invalid_values()
    {
        Assert.Throws<ArgumentException>(() => new WebhookProviderId(" "));
        Assert.Throws<ArgumentException>(() => new WebhookEventId(" "));
        Assert.Throws<ArgumentException>(() => new WebhookSubscriptionId(" "));
        Assert.Throws<ArgumentException>(() => new WebhookDeliveryId(" "));
    }

    [Fact]
    public void Subscription_rejects_empty_event_types()
    {
        Assert.Throws<ArgumentException>(() => WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "ok", " " }, true, new WebhookRetryPolicy(1, TimeSpan.FromSeconds(1), TimeSpan.FromSeconds(2))));
    }
}
