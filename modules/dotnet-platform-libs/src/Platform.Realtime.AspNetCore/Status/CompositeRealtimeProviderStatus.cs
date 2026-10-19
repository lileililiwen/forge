using System.Collections.Immutable;
using Platform.Realtime.Status;

namespace Platform.Realtime.AspNetCore.Status;

/// <summary>
/// Aggregates the safe provider status for the registered realtime transports.
/// It reports availability and backplane usage but never topology or credentials.
/// </summary>
public sealed class CompositeRealtimeProviderStatus : IRealtimeProviderStatus
{
    private readonly bool _signalREnabled;
    private readonly bool _backplaneEnabled;

    /// <summary>
    /// Initializes a new composite status snapshot.
    /// </summary>
    /// <param name="signalREnabled">Whether the SignalR transport is registered.</param>
    /// <param name="backplaneEnabled">Whether a distributed backplane is configured for SignalR.</param>
    public CompositeRealtimeProviderStatus(bool signalREnabled, bool backplaneEnabled)
    {
        _signalREnabled = signalREnabled;
        _backplaneEnabled = backplaneEnabled;
    }

    /// <inheritdoc />
    public ValueTask<ImmutableArray<RealtimeProviderStatus>> GetStatusAsync(
        CancellationToken cancellationToken = default)
    {
        var builder = ImmutableArray.CreateBuilder<RealtimeProviderStatus>();
        builder.Add(new RealtimeProviderStatus("SSE", Available: true, BackplaneEnabled: false));

        if (_signalREnabled)
        {
            builder.Add(new RealtimeProviderStatus(
                "SignalR",
                Available: true,
                BackplaneEnabled: _backplaneEnabled,
                Detail: _backplaneEnabled ? "distributed backplane" : "in-process only"));
        }

        return new ValueTask<ImmutableArray<RealtimeProviderStatus>>(builder.ToImmutable());
    }
}
