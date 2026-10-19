namespace Platform.Realtime.Authorization;

/// <summary>
/// Fail-closed default authorizer. Until an application registers its own
/// <see cref="IRealtimeConnectionAuthorizer"/>, every realtime connection is
/// rejected so a misconfigured host cannot expose streams to anonymous callers.
/// </summary>
public sealed class DenyAllRealtimeAuthorizer : IRealtimeConnectionAuthorizer
{
    /// <inheritdoc />
    public Task<RealtimeAuthorizationResult> AuthorizeAsync(
        RealtimeConnectionRequest request,
        CancellationToken cancellationToken = default) =>
        Task.FromResult(RealtimeAuthorizationResult.Deny("Realtime connections are not authorized by default."));
}
