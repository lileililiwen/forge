namespace Platform.Realtime.Delivery;

/// <summary>
/// A client-initiated request to resynchronize state after a transport
/// interruption. The platform does not guarantee delivery of messages sent
/// while a client was disconnected; applications handle replay explicitly using
/// this request as the signal.
/// </summary>
public sealed class RealtimeResyncRequest
{
    /// <summary>Gets or sets the connection that is requesting resynchronization.</summary>
    public string ConnectionId { get; init; } = string.Empty;

    /// <summary>Gets or sets an opaque resume token the client last received, if any.</summary>
    public string? LastReceivedToken { get; init; }

    /// <summary>Gets or sets the optional tenant scope the resync applies to.</summary>
    public string? TenantId { get; init; }
}
