namespace Platform.Eventing.Contracts;

/// <summary>Persistable record used to suppress duplicate event processing.</summary>
public sealed record InboxMessage
{
    private InboxMessage(
        DurableEventEnvelope envelope,
        DateTimeOffset receivedAt,
        DurableMessageState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        DurableDispatchFailure? lastFailure)
    {
        Envelope = envelope;
        ReceivedAt = receivedAt;
        State = state;
        AttemptCount = attemptCount;
        NextAttemptAt = nextAttemptAt;
        LeaseOwnerId = leaseOwnerId;
        LeaseExpiresAt = leaseExpiresAt;
        LastFailure = lastFailure;
    }

    /// <summary>Creates a pending inbox message.</summary>
    public static InboxMessage Create(DurableEventEnvelope envelope, DateTimeOffset receivedAt)
    {
        ArgumentNullException.ThrowIfNull(envelope);
        return new InboxMessage(envelope, receivedAt, DurableMessageState.Pending, 0, null, null, null, null);
    }

    /// <summary>Gets the original event envelope.</summary>
    public DurableEventEnvelope Envelope { get; }
    /// <summary>Gets the stable message identifier.</summary>
    public string MessageId => Envelope.MessageId;
    /// <summary>Gets the receive time.</summary>
    public DateTimeOffset ReceivedAt { get; }
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
    public InboxMessage WithPersistedState(
        DurableMessageState state,
        int attemptCount,
        DateTimeOffset? nextAttemptAt,
        string? leaseOwnerId,
        DateTimeOffset? leaseExpiresAt,
        DurableDispatchFailure? lastFailure) =>
        new(Envelope, ReceivedAt, state, attemptCount, nextAttemptAt, leaseOwnerId, leaseExpiresAt, lastFailure);
}
