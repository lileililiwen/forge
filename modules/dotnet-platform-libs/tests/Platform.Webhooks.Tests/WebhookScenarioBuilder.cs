using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;

namespace Platform.Webhooks.Tests;

/// <summary>Deterministic builders used across webhook tests.</summary>
public sealed class WebhookScenarioBuilder
{
    /// <summary>Scenario clock.</summary>
    public IClock Clock { get; set; } = new FixedClock(new DateTimeOffset(2026, 1, 15, 0, 0, 0, TimeSpan.Zero));
    /// <summary>Scenario webhook provider.</summary>
    public WebhookProviderId Provider { get; set; } = new("test");
    /// <summary>Scenario event identifier.</summary>
    public string EventId { get; set; } = "evt_1";
    /// <summary>Scenario subscription identifier.</summary>
    public WebhookSubscriptionId SubscriptionId { get; set; } = new("sub_1");
    /// <summary>Scenario target URI.</summary>
    public Uri Target { get; set; } = new("https://example.invalid/webhook");
    /// <summary>Scenario options.</summary>
    public WebhookOptions Options { get; set; } = new();
}
