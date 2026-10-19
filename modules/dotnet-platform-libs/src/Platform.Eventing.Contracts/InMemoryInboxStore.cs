using Platform.Core.Time;

namespace Platform.Eventing.Contracts;

/// <summary>Thread-safe in-memory inbox for development and deterministic tests.</summary>
public sealed class InMemoryInboxStore : IInboxStore
{
    private readonly object _gate = new();
    private readonly Dictionary<string, InboxMessage> _messages = new(StringComparer.Ordinal);
    private readonly IClock _clock;
    private readonly DurableEventingOptions _options;

    /// <summary>Creates an in-memory store.</summary>
    public InMemoryInboxStore(IClock clock, DurableEventingOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(clock);
        _clock = clock;
        _options = options ?? new DurableEventingOptions();
        _options.Validate();
    }

    /// <inheritdoc />
    public Task<InboxClaimResult> TryClaimAsync(InboxMessage message, DateTimeOffset now, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("Lease owner id is required.", nameof(leaseOwnerId));
        ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(leaseDuration, TimeSpan.Zero, nameof(leaseDuration));
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            if (_messages.TryGetValue(message.MessageId, out var current))
            {
                if (current.State == DurableMessageState.Succeeded) return Task.FromResult(new InboxClaimResult(InboxClaimDecision.Duplicate, current));
                if (current.State == DurableMessageState.Leased && current.LeaseExpiresAt > now)
                    return Task.FromResult(new InboxClaimResult(InboxClaimDecision.Busy, current));
                message = current;
            }
            var claimed = message.WithPersistedState(DurableMessageState.Leased, message.AttemptCount + 1, null, leaseOwnerId, now.Add(leaseDuration), message.LastFailure);
            _messages[message.MessageId] = claimed;
            return Task.FromResult(new InboxClaimResult(InboxClaimDecision.Claimed, claimed));
        }
    }

    /// <inheritdoc />
    public Task MarkSucceededAsync(string messageId, string leaseOwnerId, CancellationToken cancellationToken = default)
    {
        Update(messageId, leaseOwnerId, current => current.WithPersistedState(DurableMessageState.Succeeded, current.AttemptCount, null, null, null, current.LastFailure), cancellationToken);
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public Task MarkFailedAsync(string messageId, string leaseOwnerId, DurableDispatchFailure failure, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(failure);
        Update(messageId, leaseOwnerId, current =>
        {
            var deadLetter = failure.Permanent || current.AttemptCount >= _options.MaxAttempts;
            return current.WithPersistedState(deadLetter ? DurableMessageState.DeadLetter : DurableMessageState.RetryableFailure, current.AttemptCount, deadLetter ? null : _clock.UtcNow.Add(_options.GetRetryDelay(current.AttemptCount)), null, null, failure);
        }, cancellationToken);
        return Task.CompletedTask;
    }

    /// <inheritdoc />
    public Task<InboxMessage?> GetAsync(string messageId, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate) return Task.FromResult(_messages.GetValueOrDefault(messageId));
    }

    private void Update(string messageId, string leaseOwnerId, Func<InboxMessage, InboxMessage> update, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(messageId)) throw new ArgumentException("Message id is required.", nameof(messageId));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("Lease owner id is required.", nameof(leaseOwnerId));
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            if (!_messages.TryGetValue(messageId, out var current)) throw new KeyNotFoundException(messageId);
            if (current.State != DurableMessageState.Leased || !string.Equals(current.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal))
                throw new InvalidOperationException("The message is not leased by this worker.");
            _messages[messageId] = update(current);
        }
    }
}
