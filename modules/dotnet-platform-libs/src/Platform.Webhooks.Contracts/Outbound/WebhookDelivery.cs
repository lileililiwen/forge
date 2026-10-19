using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Outbound;

/// <summary>Persisted outbound delivery record.</summary>
public sealed record WebhookDelivery
{
    private WebhookDelivery(
        WebhookDeliveryId id,
        WebhookSubscriptionId subscriptionId,
        string eventType,
        string eventId,
        DateTimeOffset createdAt,
        WebhookDeliveryStatus status,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        WebhookFailure? lastFailure,
        int? lastResponseStatus)
    {
        Id = id; SubscriptionId = subscriptionId; EventType = eventType; EventId = eventId; CreatedAt = createdAt;
        Status = status; AttemptCount = attemptCount; NextAttemptAt = nextAttemptAt; LastFailure = lastFailure; LastResponseStatus = lastResponseStatus;
    }
    /// <summary>Creates a fresh pending delivery.</summary>
    public static WebhookDelivery Create(WebhookDeliveryId id, WebhookSubscriptionId subscriptionId, string eventType, string eventId, DateTimeOffset createdAt)
    {
        if (string.IsNullOrWhiteSpace(eventType)) throw new ArgumentException("Event types are required.", nameof(eventType));
        if (string.IsNullOrWhiteSpace(eventId)) throw new ArgumentException("Event identifiers are required.", nameof(eventId));
        return new WebhookDelivery(id, subscriptionId, eventType, eventId, createdAt, WebhookDeliveryStatus.Pending, 0, null, null, null);
    }
    /// <summary>Rehydrates a stored delivery with the supplied persisted state.</summary>
    public WebhookDelivery WithPersistedState(
        WebhookDeliveryStatus status,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        WebhookFailure? lastFailure,
        int? lastResponseStatus) =>
        new(Id, SubscriptionId, EventType, EventId, CreatedAt, status, attemptCount, nextAttemptAt, lastFailure, lastResponseStatus);
    /// <summary>Delivery identifier.</summary>
    public WebhookDeliveryId Id { get; }
    /// <summary>Subscription identifier.</summary>
    public WebhookSubscriptionId SubscriptionId { get; }
    /// <summary>Event type for routing and metadata.</summary>
    public string EventType { get; }
    /// <summary>Stable event identifier.</summary>
    public string EventId { get; }
    /// <summary>Time the delivery was created.</summary>
    public DateTimeOffset CreatedAt { get; }
    /// <summary>Current state.</summary>
    public WebhookDeliveryStatus Status { get; }
    /// <summary>Number of attempts so far.</summary>
    public int AttemptCount { get; }
    /// <summary>Next eligible attempt time.</summary>
    public DateTimeOffset? NextAttemptAt { get; }
    /// <summary>Last safe failure.</summary>
    public WebhookFailure? LastFailure { get; }
    /// <summary>Last response status code observed.</summary>
    public int? LastResponseStatus { get; }
}
