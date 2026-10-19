using Platform.Realtime.Delivery;

namespace Platform.Realtime.AspNetCore.Delivery;

/// <summary>
/// Application-owned handler for a client resynchronization request. Because realtime
/// delivery is non-durable, the application decides how to replay or resync state
/// after a transport interruption. The platform only routes the request.
/// </summary>
public interface IRealtimeResyncHandler
{
    /// <summary>
    /// Handles a resynchronization request from a client that reconnected after a
    /// transport interruption.
    /// </summary>
    /// <param name="request">The resync request.</param>
    /// <param name="cancellationToken">A token that cancels the handling.</param>
    Task HandleAsync(RealtimeResyncRequest request, CancellationToken cancellationToken = default);
}
