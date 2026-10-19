using Platform.Webhooks.Contracts.Outbound;

using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Thread-safe in-memory subscription store for local use and deterministic tests.</summary>
public sealed class InMemoryWebhookSubscriptionStore : IWebhookSubscriptionStore
{
    private readonly object _gate = new();
    private readonly Dictionary<WebhookSubscriptionId, WebhookSubscription> _subscriptions = new();

    /// <summary>Adds or replaces a subscription.</summary>
    public void Upsert(WebhookSubscription subscription)
    {
        ArgumentNullException.ThrowIfNull(subscription);
        lock (_gate) _subscriptions[subscription.Id] = subscription;
    }

    /// <inheritdoc />
    public Task<WebhookSubscription?> GetAsync(WebhookSubscriptionId id, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) return Task.FromResult(_subscriptions.TryGetValue(id, out var sub) ? sub : null);
    }

    /// <inheritdoc />
    public Task<IReadOnlyList<WebhookSubscription>> ListEnabledAsync(string eventType, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            IReadOnlyList<WebhookSubscription> list = _subscriptions.Values
                .Where(sub => sub.IsEnabled && (sub.EventTypes.Count == 0 || sub.EventTypes.Contains(eventType)))
                .ToArray();
            return Task.FromResult(list);
        }
    }
}
