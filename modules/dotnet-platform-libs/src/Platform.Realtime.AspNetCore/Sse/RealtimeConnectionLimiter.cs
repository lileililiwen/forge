using Microsoft.Extensions.Options;
using Platform.Realtime;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// Bounded, thread-safe counter of active realtime connections. The adapter
/// acquires a slot before streaming and releases it in a <c>finally</c> block so
/// a disconnect or cancellation never leaks a slot. When the limit is reached the
/// next connection is rejected (HTTP 503) rather than queued without bound.
/// </summary>
public sealed class RealtimeConnectionLimiter
{
    private int _active;

    /// <summary>Gets the configured maximum number of concurrent connections.</summary>
    public int Max { get; }

    /// <summary>Initializes a new limiter from the realtime connection options.</summary>
    public RealtimeConnectionLimiter(IOptions<RealtimeConnectionOptions> options)
    {
        ArgumentNullException.ThrowIfNull(options);
        Max = Math.Max(1, options.Value.MaxConcurrentConnections);
    }

    /// <summary>Gets the current number of active connections.</summary>
    public int ActiveCount => Volatile.Read(ref _active);

    /// <summary>
    /// Attempts to acquire a connection slot. Returns <c>false</c> when the
    /// configured maximum is already in use.
    /// </summary>
    public bool TryAcquire()
    {
        var current = Interlocked.Increment(ref _active);
        if (current > Max)
        {
            Interlocked.Decrement(ref _active);
            return false;
        }

        return true;
    }

    /// <summary>Releases a previously acquired connection slot.</summary>
    public void Release()
    {
        var value = Interlocked.Decrement(ref _active);
        if (value < 0) Interlocked.Exchange(ref _active, 0);
    }
}
