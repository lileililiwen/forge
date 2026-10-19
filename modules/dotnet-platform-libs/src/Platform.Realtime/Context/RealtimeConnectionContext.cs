using Platform.Core.Context;

namespace Platform.Realtime.Context;

/// <summary>
/// Runtime view of an accepted realtime connection handed to application
/// stream handlers. Carries the caller context and the cancellation token that
/// fires when the client disconnects or the request is aborted.
/// </summary>
public sealed class RealtimeConnectionContext
{
    /// <summary>Gets or sets the transport-assigned connection identifier.</summary>
    public string ConnectionId { get; init; } = string.Empty;

    /// <summary>Gets or sets the resolved caller context for this connection.</summary>
    public CallerContext Caller { get; init; } = CallerContext.Anonymous;

    /// <summary>Gets or sets the tenant of the caller, when one was resolved.</summary>
    public string? TenantId => Caller.TenantId;

    /// <summary>Gets or sets the token that cancels when the client disconnects.</summary>
    public CancellationToken Cancellation { get; init; }
}
