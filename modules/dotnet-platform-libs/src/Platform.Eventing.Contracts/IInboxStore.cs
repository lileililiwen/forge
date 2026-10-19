namespace Platform.Eventing.Contracts;

/// <summary>Result of attempting to claim an inbox message.</summary>
public enum InboxClaimDecision
{
    /// <summary>The message was claimed by this worker.</summary>
    Claimed,
    /// <summary>The message was already completed.</summary>
    Duplicate,
    /// <summary>The message is currently leased by another worker.</summary>
    Busy,
}

/// <summary>Inbox claim result and, when claimed, the owned record.</summary>
public sealed record InboxClaimResult(InboxClaimDecision Decision, InboxMessage? Message);

/// <summary>Application-replaceable durable inbox store.</summary>
public interface IInboxStore
{
    /// <summary>Claims a new or retryable message, suppressing completed duplicates.</summary>
    Task<InboxClaimResult> TryClaimAsync(InboxMessage message, DateTimeOffset now, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default);
    /// <summary>Marks a worker-owned message as successfully processed.</summary>
    Task MarkSucceededAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken = default);
    /// <summary>Records a worker-owned failure and schedules retry or dead-letter state.</summary>
    Task MarkFailedAsync(string messageId, string leaseOwnerId, DurableDispatchFailure failure, CancellationToken cancellationToken = default);
    /// <summary>Gets a message by identifier.</summary>
    Task<InboxMessage?> GetAsync(string messageId, CancellationToken cancellationToken = default);
}
