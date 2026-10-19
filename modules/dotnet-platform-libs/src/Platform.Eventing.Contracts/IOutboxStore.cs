namespace Platform.Eventing.Contracts;

/// <summary>Application-replaceable durable outbox store.</summary>
public interface IOutboxStore
{
    /// <summary>Adds a message if its identifier is not already present.</summary>
    Task AddAsync(OutboxMessage message, CancellationToken cancellationToken = default);
    /// <summary>Claims eligible messages for a worker.</summary>
    Task<IReadOnlyList<OutboxMessage>> ClaimAsync(DateTimeOffset now, string leaseOwnerId, TimeSpan leaseDuration, int maximum, CancellationToken cancellationToken = default);
    /// <summary>Marks a worker-owned message as successfully published.</summary>
    Task MarkSucceededAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken = default);
    /// <summary>Records a worker-owned failure and schedules retry or dead-letter state.</summary>
    Task MarkFailedAsync(string messageId, string leaseOwnerId, DurableDispatchFailure failure, CancellationToken cancellationToken = default);
    /// <summary>Gets a message by identifier.</summary>
    Task<OutboxMessage?> GetAsync(string messageId, CancellationToken cancellationToken = default);
}
