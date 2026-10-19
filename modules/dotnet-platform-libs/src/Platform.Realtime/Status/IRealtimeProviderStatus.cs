using System.Collections.Immutable;

namespace Platform.Realtime.Status;

/// <summary>
/// Exposes safe provider health for the realtime transports without leaking
/// topology or credentials. Hosts surface this through a status endpoint rather
/// than relying on transport internals.
/// </summary>
public interface IRealtimeProviderStatus
{
    /// <summary>
    /// Returns the current safe status for every registered transport.
    /// </summary>
    /// <param name="cancellationToken">A token that cancels the read.</param>
    /// <returns>An immutable set of transport statuses.</returns>
    ValueTask<ImmutableArray<RealtimeProviderStatus>> GetStatusAsync(
        CancellationToken cancellationToken = default);
}
