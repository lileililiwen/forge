using System.Collections.Concurrent;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;

namespace Platform.Webhooks.Contracts.Inbound;

/// <summary>Thread-safe, in-memory inbound inbox store for local use and deterministic tests.</summary>
public sealed class InMemoryWebhookInboxStore : IWebhookInboxStore
{
    private readonly object _gate = new();
    private readonly Dictionary<string, WebhookInboxMessage> _messages = new(StringComparer.Ordinal);
    private readonly WebhookOptions _options;
    private readonly int _maxAttempts;

    /// <summary>Creates an empty inbox store.</summary>
    public InMemoryWebhookInboxStore(WebhookOptions? options = null, int maxAttempts = 5)
    {
        _options = options ?? new WebhookOptions();
        _options.Validate();
        if (maxAttempts < 1 || maxAttempts > 32) throw new ArgumentOutOfRangeException(nameof(maxAttempts));
        _maxAttempts = maxAttempts;
    }

    /// <inheritdoc />
    public Task<WebhookInboxClaimResult> TryClaimAsync(WebhookInboxMessage message, IClock clock, string leaseOwnerId, TimeSpan leaseDuration, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        ArgumentNullException.ThrowIfNull(message);
        ArgumentNullException.ThrowIfNull(clock);
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        if (leaseDuration <= TimeSpan.Zero || leaseDuration > TimeSpan.FromHours(24)) throw new ArgumentOutOfRangeException(nameof(leaseDuration));
        var now = clock.UtcNow;
        lock (_gate)
        {
            if (_messages.TryGetValue(message.ReplayKey, out var existing))
            {
                if (existing.State == WebhookInboxState.Completed) return Task.FromResult(new WebhookInboxClaimResult(WebhookInboxClaimStatus.Duplicate, existing));
                if (existing.State == WebhookInboxState.Leased && existing.LeaseExpiresAt is { } lease && lease > now && !string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal))
                    return Task.FromResult(new WebhookInboxClaimResult(WebhookInboxClaimStatus.Busy, existing));
                if (existing.NextAttemptAt is { } next && next > now) return Task.FromResult(new WebhookInboxClaimResult(WebhookInboxClaimStatus.Busy, existing));
                var leaseExpiry = now.Add(leaseDuration);
                var updated = existing.WithPersistedState(WebhookInboxState.Leased, existing.AttemptCount + 1, null, leaseOwnerId, leaseExpiry, null);
                _messages[message.ReplayKey] = updated;
                return Task.FromResult(new WebhookInboxClaimResult(WebhookInboxClaimStatus.Claimed, updated));
            }
            var leaseExpiryFresh = now.Add(leaseDuration);
            var stored = message.WithPersistedState(WebhookInboxState.Leased, 1, null, leaseOwnerId, leaseExpiryFresh, null);
            _messages.Add(message.ReplayKey, stored);
            return Task.FromResult(new WebhookInboxClaimResult(WebhookInboxClaimStatus.Claimed, stored));
        }
    }

    /// <inheritdoc />
    public Task MarkCompletedAsync(string replayKey, string leaseOwnerId, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (string.IsNullOrWhiteSpace(replayKey)) throw new ArgumentException("A replay key is required.", nameof(replayKey));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        lock (_gate)
        {
            if (!_messages.TryGetValue(replayKey, out var existing)) return Task.CompletedTask;
            if (!string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal)) throw new InvalidOperationException("The lease owner does not match the current lease.");
            _messages[replayKey] = existing.WithPersistedState(WebhookInboxState.Completed, existing.AttemptCount, null, null, null, null);
            return Task.CompletedTask;
        }
    }

    /// <inheritdoc />
    public Task MarkFailedAsync(string replayKey, string leaseOwnerId, WebhookFailure failure, DateTimeOffset? nextAttemptAt, bool deadLettered, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (string.IsNullOrWhiteSpace(replayKey)) throw new ArgumentException("A replay key is required.", nameof(replayKey));
        if (string.IsNullOrWhiteSpace(leaseOwnerId)) throw new ArgumentException("A lease owner is required.", nameof(leaseOwnerId));
        ArgumentNullException.ThrowIfNull(failure);
        lock (_gate)
        {
            if (!_messages.TryGetValue(replayKey, out var existing)) return Task.CompletedTask;
            if (!string.Equals(existing.LeaseOwnerId, leaseOwnerId, StringComparison.Ordinal)) throw new InvalidOperationException("The lease owner does not match the current lease.");
            var attempts = existing.AttemptCount;
            var state = deadLettered || attempts >= _maxAttempts ? WebhookInboxState.DeadLettered : WebhookInboxState.Pending;
            _messages[replayKey] = existing.WithPersistedState(state, attempts, deadLettered ? null : nextAttemptAt, deadLettered ? null : leaseOwnerId, deadLettered ? null : existing.LeaseExpiresAt, failure);
            return Task.CompletedTask;
        }
    }

    /// <inheritdoc />
    public Task<WebhookInboxMessage?> GetAsync(string replayKey, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        lock (_gate)
        {
            return Task.FromResult(_messages.TryGetValue(replayKey, out var existing) ? existing : null);
        }
    }

    /// <summary>Returns the number of tracked messages for inspection and reconciliation.</summary>
    public int Count
    {
        get { lock (_gate) return _messages.Count; }
    }
}
