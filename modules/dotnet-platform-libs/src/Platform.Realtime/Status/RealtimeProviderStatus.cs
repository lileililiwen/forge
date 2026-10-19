namespace Platform.Realtime.Status;

/// <summary>
/// Safe, topology-free health snapshot for a realtime transport. It reports
/// availability and whether a distributed backplane is in use, but never
/// exposes connection strings, Redis topology, or credentials.
/// </summary>
/// <param name="Transport">The transport name, e.g. <c>SSE</c> or <c>SignalR</c>.</param>
/// <param name="Available">Whether the transport is currently able to serve connections.</param>
/// <param name="BackplaneEnabled">Whether a distributed backplane is configured.</param>
/// <param name="Detail">Optional, non-sensitive status detail.</param>
public readonly record struct RealtimeProviderStatus(
    string Transport,
    bool Available,
    bool BackplaneEnabled,
    string? Detail = null)
{
    /// <summary>An unavailable status for a transport that is not registered.</summary>
    public static RealtimeProviderStatus Unavailable(string transport) =>
        new(transport, Available: false, BackplaneEnabled: false, Detail: "not registered");
}
