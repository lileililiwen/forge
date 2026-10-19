using Platform.Realtime.Messages;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// Platform-enforced sink for SSE messages. Implementations validate payload
/// bounds and apply the application tenant-routing decision before any bytes are
/// written to the client, so a rejected cross-tenant message is silently
/// filtered rather than broadcast.
/// </summary>
public interface IRealtimeMessageSink
{
    /// <summary>
    /// Writes a message to the connection, applying payload and tenant-routing
    /// enforcement. Messages rejected by routing are dropped without error.
    /// </summary>
    /// <param name="message">The message to deliver.</param>
    /// <param name="cancellationToken">A token that cancels the write.</param>
    /// <returns>A task representing the asynchronous write.</returns>
    ValueTask WriteAsync(RealtimeMessage message, CancellationToken cancellationToken = default);
}
