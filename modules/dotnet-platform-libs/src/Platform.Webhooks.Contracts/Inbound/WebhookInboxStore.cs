using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Persisted inbound webhook message used to suppress replays.</summary>
public sealed record WebhookInboxMessage
{
    private WebhookInboxMessage(
        WebhookProviderId provider,
        string eventId,
        DateTimeOffset receivedAt,
        WebhookInboxState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        WebhookFailure? lastFailure)
    {
        Provider = provider; EventId = eventId; ReceivedAt = receivedAt; State = state; AttemptCount = attemptCount;
        NextAttemptAt = nextAttemptAt; LeaseOwnerId = leaseOwnerId; LeaseExpiresAt = leaseExpiresAt; LastFailure = lastFailure;
    }
    /// <summary>Creates a fresh pending message.</summary>
    public static WebhookInboxMessage Create(WebhookProviderId provider, string eventId, DateTimeOffset receivedAt)
    {
        if (string.IsNullOrWhiteSpace(eventId)) throw new ArgumentException("Event identifiers are required.", nameof(eventId));
        return new WebhookInboxMessage(provider, eventId, receivedAt, WebhookInboxState.Pending, 0, null, null, null, null);
    }
    /// <summary>Rehydrates a stored message with the supplied persisted state.</summary>
    public WebhookInboxMessage WithPersistedState(
        WebhookInboxState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        WebhookFailure? lastFailure) =>
        new(Provider, EventId, ReceivedAt, state, attemptCount, nextAttemptAt, leaseOwnerId, leaseExpiresAt, lastFailure);
    /// <summary>Provider scope.</summary>
    public WebhookProviderId Provider { get; }
    /// <summary>Event identifier.</summary>
    public string EventId { get; }
    /// <summary>Receive time.</summary>
    public DateTimeOffset ReceivedAt { get; }
    /// <summary>Current state.</summary>
    public WebhookInboxState State { get; }
    /// <summary>Number of attempts.</summary>
    public int AttemptCount { get; }
    /// <summary>Next eligible attempt time.</summary>
    public DateTimeOffset? NextAttemptAt { get; }
    /// <summary>Current lease owner.</summary>
    public string? LeaseOwnerId { get; }
    /// <summary>Current lease expiry.</summary>
    public DateTimeOffset? LeaseExpiresAt { get; }
    /// <summary>Last safe failure.</summary>
    public WebhookFailure? LastFailure { get; }
    /// <summary>Stable composite key used to deduplicate.</summary>
    public string ReplayKey => Provider.Value + "\u001f" + EventId;
}

/// <summary>Replay-keyed claim result.</summary>
public sealed record WebhookInboxClaimResult(WebhookInboxClaimStatus Status, WebhookInboxMessage? Message, WebhookFailure? Failure = null);

/// <summary>Persistable inbound replay store boundary.</summary>
public interface IWebhookInboxStore
{
    /// <summary>Claims a new or retryable message, suppressing completed duplicates.</summary>
    Task<WebhookInboxClaimResult> TryClaimAsync(WebhookInboxMessage message, IClock clock, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default);
    /// <summary>Marks a leased message as successfully processed.</summary>
    Task MarkCompletedAsync(string replayKey, string leaseOwnerId, CancellationToken cancellationToken = default);
    /// <summary>Records a leased failure and schedules retry or dead-letter state.</summary>
    Task MarkFailedAsync(string replayKey, string leaseOwnerId, WebhookFailure failure, DateTimeOffset? nextAttemptAt, bool deadLettered, CancellationToken cancellationToken = default);
    /// <summary>Reads a message by replay key.</summary>
    Task<WebhookInboxMessage?> GetAsync(string replayKey, CancellationToken cancellationToken = default);
}
