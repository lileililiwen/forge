using System.Text;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.Contracts.Security;

namespace Platform.Webhooks.Tests.Outbound;

public sealed class WebhookOutboundDispatcherTests
{
    [Fact]
    public async Task Successful_delivery_records_succeeded_state()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "order.created" }, true, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        secrets.Add("default", Encoding.UTF8.GetBytes("super-secret"));
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.Success(200));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{\"id\":\"evt_1\"}");

        Assert.Equal(WebhookDeliveryOutcome.Succeeded, result.Outcome);
        var stored = await deliveries.GetAsync(new WebhookDeliveryId("d_1"));
        Assert.Equal(WebhookDeliveryStatus.Succeeded, stored!.Status);
        Assert.Equal(200, stored.LastResponseStatus);
    }

    [Fact]
    public async Task Private_target_is_rejected_with_safe_failure()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://10.0.0.1/hook"), "default", new[] { "order.created" }, true, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        secrets.Add("default", Encoding.UTF8.GetBytes("super-secret"));
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.Success(200));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{\"id\":\"evt_1\"}");

        Assert.Equal(WebhookDeliveryOutcome.Rejected, result.Outcome);
        Assert.Equal("webhook.ssrf_rejected", result.Failure!.Code);
    }

    [Fact]
    public async Task Retryable_5xx_schedules_retry()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "order.created" }, true, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        secrets.Add("default", Encoding.UTF8.GetBytes("super-secret"));
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.ResponseFailure(503, new WebhookFailure("webhook.target_unavailable", "Target unavailable.", true)));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{\"id\":\"evt_1\"}");

        Assert.Equal(WebhookDeliveryOutcome.RetryableResponse, result.Outcome);
        Assert.NotNull(result.NextAttemptAt);
        var stored = await deliveries.GetAsync(new WebhookDeliveryId("d_1"));
        Assert.Equal(WebhookDeliveryStatus.RetryScheduled, stored!.Status);
    }

    [Fact]
    public async Task Permanent_4xx_response_marks_rejected()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "order.created" }, true, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        secrets.Add("default", Encoding.UTF8.GetBytes("super-secret"));
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.ResponseFailure(400, new WebhookFailure("webhook.rejected_by_target", "Bad request.", false)));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{\"id\":\"evt_1\"}");

        Assert.Equal(WebhookDeliveryOutcome.PermanentResponse, result.Outcome);
        var stored = await deliveries.GetAsync(new WebhookDeliveryId("d_1"));
        Assert.Equal(WebhookDeliveryStatus.Rejected, stored!.Status);
    }

    [Fact]
    public async Task Transport_failure_classifies_transiently()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "order.created" }, true, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        secrets.Add("default", Encoding.UTF8.GetBytes("super-secret"));
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.TransportFailure(new WebhookFailure("webhook.transport_error", "Connection refused.", true)));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{\"id\":\"evt_1\"}");

        Assert.Equal(WebhookDeliveryOutcome.TransportError, result.Outcome);
        Assert.True(result.Failure!.Transient);
    }

    [Fact]
    public async Task Unknown_subscription_is_rejected()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.Success(200));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(new WebhookSubscriptionId("missing"), "order.created", "evt_1", "{}");

        Assert.Equal(WebhookDeliveryOutcome.Rejected, result.Outcome);
        Assert.Equal("webhook.subscription_not_found", result.Failure!.Code);
    }

    [Fact]
    public async Task Disabled_subscription_is_rejected()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var subscriptions = new InMemoryWebhookSubscriptionStore();
        var deliveries = new InMemoryWebhookDeliveryStore();
        var secrets = new ConfigurationWebhookSigningSecretResolver();
        var sub = WebhookSubscription.Create(new WebhookSubscriptionId("sub_1"), new Uri("https://203.0.113.10/hook"), "default", new[] { "order.created" }, false, new WebhookRetryPolicy(3, TimeSpan.FromSeconds(1), TimeSpan.FromMinutes(1)));
        subscriptions.Upsert(sub);
        var validator = new SsrfTargetValidator(new WebhookOptions());
        var sender = new TestSender((req, _) => WebhookHttpSendResult.Success(200));
        var dispatcher = new WebhookOutboundDispatcher(subscriptions, deliveries, secrets, validator, sender, Options.Create(new WebhookOptions()), clock, () => new WebhookDeliveryId("d_1"));

        var result = await dispatcher.DispatchAsync(sub.Id, "order.created", "evt_1", "{}");

        Assert.Equal(WebhookDeliveryOutcome.Rejected, result.Outcome);
        Assert.Equal("webhook.subscription_disabled", result.Failure!.Code);
    }

    private sealed class TestSender : IWebhookHttpSender
    {
        private readonly Func<WebhookHttpSendRequest, CancellationToken, WebhookHttpSendResult> _handler;
        public TestSender(Func<WebhookHttpSendRequest, CancellationToken, WebhookHttpSendResult> handler) => _handler = handler;
        public ValueTask<WebhookHttpSendResult> SendAsync(WebhookHttpSendRequest request, CancellationToken cancellationToken = default) => new(_handler(request, cancellationToken));
    }
}
