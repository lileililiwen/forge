using Microsoft.AspNetCore.SignalR;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// Configuration for the platform SignalR realtime adapter. The backplane is
/// intentionally application-owned: the host supplies the configuration delegate
/// that adds Redis or another scale-out transport. When no delegate is supplied
/// the host runs SignalR in-process (no-backplane mode).
/// </summary>
public sealed class RealtimeSignalROptions
{
    /// <summary>
    /// Gets or sets an optional delegate that configures the SignalR server builder
    /// with a distributed backplane (e.g. Redis). When <c>null</c>, no backplane is
    /// added and the host runs in-process only.
    /// </summary>
    public Action<ISignalRServerBuilder>? ConfigureBackplane { get; set; }
}
