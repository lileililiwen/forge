using Platform.Realtime.Context;
using Platform.Realtime.Messages;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// Application-owned SSE stream producer. For each accepted connection the
/// adapter invokes <see cref="StreamAsync"/> and the implementation writes
/// messages to the supplied <see cref="IRealtimeMessageSink"/> until the
/// connection is cancelled. The adapter enforces authorization, tenant routing,
/// payload limits, and heartbeats; the source only produces content.
/// </summary>
public interface IRealtimeSseSource
{
    /// <summary>
    /// Streams messages for a single connection. Implementations MUST observe
    /// <see cref="RealtimeConnectionContext.Cancellation"/> and return when it
    /// fires so the connection is released promptly.
    /// </summary>
    /// <param name="context">The accepted connection context.</param>
    /// <param name="sink">The platform-enforced message sink.</param>
    /// <param name="cancellationToken">A token that cancels when the client disconnects.</param>
    Task StreamAsync(
        RealtimeConnectionContext context,
        IRealtimeMessageSink sink,
        CancellationToken cancellationToken);
}
