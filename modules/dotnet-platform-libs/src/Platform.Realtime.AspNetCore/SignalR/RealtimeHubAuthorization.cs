using Platform.Realtime.Authorization;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// Transport-neutral authorization policy for realtime connections. Extracted from
/// the SignalR filter so the decision can be unit-tested without a live hub.
/// </summary>
public static class RealtimeHubAuthorization
{
    /// <summary>
    /// Authorizes a connection request and throws when the application decision
    /// denies it. Throwing during connection establishment forces the transport to
    /// reject the client.
    /// </summary>
    /// <param name="request">The inbound connection description.</param>
    /// <param name="authorizer">The application authorization decision point.</param>
    /// <param name="cancellationToken">A token that cancels the check.</param>
    public static async ValueTask AuthorizeOrThrowAsync(
        RealtimeConnectionRequest request,
        IRealtimeConnectionAuthorizer authorizer,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(authorizer);

        var result = await authorizer.AuthorizeAsync(request, cancellationToken).ConfigureAwait(false);
        if (!result.Allowed)
            throw new InvalidOperationException(result.RejectionReason ?? "Realtime connection rejected.");
    }
}
