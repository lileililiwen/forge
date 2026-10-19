namespace Platform.Eventing.Contracts;

/// <summary>Persistable event awaiting publication.</summary>
public sealed record OutboxMessage
{
    private OutboxMessage(
        DurableEventEnvelope envelope,
        DateTimeOffset createdAt,
        DurableMessageState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        DurableDispatchFailure? lastFailure)
    {
        Envelope = envelope;
        CreatedAt = createdAt;
        State = state;
        AttemptCount = attemptCount;
        NextAttemptAt = nextAttemptAt;
        LeaseOwnerId = leaseOwnerId;
        LeaseExpiresAt = leaseExpiresAt;
        LastFailure = lastFailure;
    }

    /// <summary>Creates a pending outbox message.</summary>
    public static OutboxMessage Create(DurableEventEnvelope envelope, DateTimeOffset createdAt)
    {
        ArgumentNullException.ThrowIfNull(envelope);
        return new OutboxMessage(envelope, createdAt, DurableMessageState.Pending, 0, null, null, null, null);
    }

    /// <summary>Gets the original event envelope.</summary>
    public DurableEventEnvelope Envelope { get; }
    /// <summary>Gets the stable message identifier.</summary>
    public string MessageId => Envelope.MessageId;
    /// <summary>Gets the creation time.</summary>
    public DateTimeOffset CreatedAt { get; }
    /// <summary>Gets the processing state.</summary>
    public DurableMessageState State { get; }
    /// <summary>Gets the number of attempted claims.</summary>
    public int AttemptCount { get; }
    /// <summary>Gets the next eligible retry time.</summary>
    public DateTimeOffset? NextAttemptAt { get; }
    /// <summary>Gets the current lease owner.</summary>
    public string? LeaseOwnerId { get; }
    /// <summary>Gets the current lease expiry.</summary>
    public DateTimeOffset? LeaseExpiresAt { get; }
    /// <summary>Gets the last safe failure.</summary>
    public DurableDispatchFailure? LastFailure { get; }

    /// <summary>Rehydrates a message returned by a durable store.</summary>
    public OutboxMessage WithPersistedState(
        DurableMessageState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        DurableDispatchFailure? lastFailure) =>
        new(Envelope, CreatedAt, state, attemptCount, nextAttemptAt, leaseOwnerId, leaseExpiresAt, lastFailure);
}
