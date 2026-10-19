namespace Platform.Realtime.Authorization;

/// <summary>
/// Application-owned decision point invoked by every realtime transport before
/// a connection is accepted. The platform never connects a client without a
/// positive result from this contract.
/// </summary>
public interface IRealtimeConnectionAuthorizer
{
    /// <summary>
    /// Returns whether the supplied connection attempt may proceed.
    /// </summary>
    /// <param name="request">The inbound connection description.</param>
    /// <param name="cancellationToken">A token that cancels the authorization check.</param>
    /// <returns>The authorization decision.</returns>
    Task<RealtimeAuthorizationResult> AuthorizeAsync(
        RealtimeConnectionRequest request,
        CancellationToken cancellationToken = default);
}
