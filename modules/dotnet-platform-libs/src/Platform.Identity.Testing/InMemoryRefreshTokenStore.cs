using System.Collections.Concurrent;
using System.Globalization;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>
/// Deterministic, linearizable in-memory refresh-token store. Not for production —
/// the platform explicitly recommends a hashed-at-rest application store that
/// survives application restarts and supports multi-instance rotation.
/// </summary>
public sealed class InMemoryRefreshTokenStore : IRefreshTokenStore
{
    private readonly ConcurrentDictionary<string, Entry> _entries = new(StringComparer.Ordinal);
    private readonly object _gate = new();
    private long _issued;

    /// <summary>The number of entries currently held in the store.</summary>
    public int Count => _entries.Count;

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<RefreshToken>> IssueAsync(string subjectId, string sessionId, DateTimeOffset expiresAt, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(subjectId))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshToken>(IdentityLifecycleOutcome.InvalidRequest));
        if (string.IsNullOrWhiteSpace(sessionId))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshToken>(IdentityLifecycleOutcome.InvalidRequest));
        var id = Interlocked.Increment(ref _issued);
        var handle = "rt_" + id.ToString("D20", CultureInfo.InvariantCulture);
        var entry = new Entry(new RefreshToken(handle, subjectId, sessionId, DateTimeOffset.UtcNow, expiresAt), id, expiresAt);
        _entries[handle] = entry;
        return ValueTask.FromResult(IdentityLifecycleResults.Success(entry.Token));
    }

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleResult<RefreshTokenRotation>> ConsumeAsync(string handle, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(handle))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshTokenRotation>(IdentityLifecycleOutcome.InvalidHandle));
        if (!_entries.TryGetValue(handle, out var entry))
            return ValueTask.FromResult(IdentityLifecycleResults.Failed<RefreshTokenRotation>(IdentityLifecycleOutcome.InvalidHandle));

        IdentityLifecycleOutcome outcome;
        RefreshTokenRotation? rotation = null;
        lock (_gate)
        {
            if (entry.Revoked)
            {
                outcome = IdentityLifecycleOutcome.Revoked;
            }
            else if (entry.Consumed)
            {
                RevokeFamily(entry);
                outcome = IdentityLifecycleOutcome.Replayed;
            }
            else if (entry.Token.ExpiresAt <= DateTimeOffset.UtcNow)
            {
                outcome = IdentityLifecycleOutcome.Expired;
            }
            else
            {
                entry.Consumed = true;
                var nextId = Interlocked.Increment(ref _issued);
                var nextHandle = "rt_" + nextId.ToString("D20", CultureInfo.InvariantCulture);
                var next = new Entry(
                    new RefreshToken(nextHandle, entry.Token.SubjectId, entry.Token.SessionId, DateTimeOffset.UtcNow, entry.Token.ExpiresAt),
                    nextId,
                    entry.Token.ExpiresAt)
                {
                    Family = entry.Family,
                    Predecessor = handle,
                };
                _entries[nextHandle] = next;
                entry.Successor = nextHandle;
                rotation = new RefreshTokenRotation(next.Token.Handle, nextHandle, next.Token.ExpiresAt);
                outcome = IdentityLifecycleOutcome.Succeeded;
            }
        }
        return ValueTask.FromResult(rotation is null
            ? IdentityLifecycleResults.Failed<RefreshTokenRotation>(outcome)
            : IdentityLifecycleResults.Success(rotation));
    }

    /// <inheritdoc />
    public ValueTask<IdentityLifecycleOutcome> RevokeAsync(string handle, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(handle))
            return ValueTask.FromResult(IdentityLifecycleOutcome.InvalidHandle);
        if (!_entries.TryGetValue(handle, out var entry))
            return ValueTask.FromResult(IdentityLifecycleOutcome.InvalidHandle);
        lock (_gate)
        {
            RevokeFamily(entry);
        }
        return ValueTask.FromResult(IdentityLifecycleOutcome.Succeeded);
    }

    private void RevokeFamily(Entry entry)
    {
        RevokeFamily(entry, new HashSet<string>(StringComparer.Ordinal));
    }

    private void RevokeFamily(Entry entry, HashSet<string> visited)
    {
        if (!visited.Add(entry.Token.Handle))
            return;
        entry.Revoked = true;
        if (!string.IsNullOrEmpty(entry.Predecessor) && _entries.TryGetValue(entry.Predecessor, out var predecessor))
            RevokeFamily(predecessor, visited);
        if (!string.IsNullOrEmpty(entry.Successor) && _entries.TryGetValue(entry.Successor, out var successor))
            RevokeFamily(successor, visited);
    }

    private sealed class Entry
    {
        public Entry(RefreshToken token, long sequence, DateTimeOffset expiresAt)
        {
            Token = token;
            Family = sequence;
            ExpiresAt = expiresAt;
        }
        public RefreshToken Token { get; }
        public long Family { get; init; }
        public DateTimeOffset ExpiresAt { get; }
        public bool Consumed;
        public bool Revoked;
        public string? Predecessor;
        public string? Successor;
    }
}
