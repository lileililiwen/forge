using Platform.Webhooks.Contracts.Outbound;

using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Thread-safe in-memory delivery store for local use and deterministic tests.</summary>
public sealed class InMemoryWebhookDeliveryStore : IWebhookDeliveryStore
{
    private readonly object _gate = new();
    private readonly Dictionary<WebhookDeliveryId, WebhookDelivery> _deliveries = new();

    /// <inheritdoc />
    public Task<WebhookDelivery> CreateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(delivery);
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) { _deliveries[delivery.Id] = delivery; }
        return Task.FromResult(delivery);
    }

    /// <inheritdoc />
    public Task UpdateAsync(WebhookDelivery delivery, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(delivery);
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) { _deliveries[delivery.Id] = delivery; }
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public Task<WebhookDelivery?> GetAsync(WebhookDeliveryId id, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) return Task.FromResult(_deliveries.TryGetValue(id, out var value) ? value : null);
    }
}
