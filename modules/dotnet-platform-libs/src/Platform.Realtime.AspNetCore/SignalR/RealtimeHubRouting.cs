using Platform.Realtime.Tenant;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// Transport-neutral tenant-routing check for realtime delivery. Extracted from
/// the hub base so the policy can be unit-tested without a live hub.
/// </summary>
public static class RealtimeHubRouting
{
    /// <summary>
    /// Returns whether the candidate <paramref name="route"/> may be delivered given
    /// the caller and the application routing policy.
    /// </summary>
    /// <param name="route">The candidate delivery route.</param>
    /// <param name="router">The application tenant-routing decision point.</param>
    /// <param name="cancellationToken">A token that cancels the check.</param>
    public static ValueTask<bool> IsRouteAllowedAsync(
        RealtimeTenantRoute route,
        IRealtimeTenantRouter router,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(route);
        ArgumentNullException.ThrowIfNull(router);
        return router.IsRouteAllowedAsync(route, cancellationToken);
    }
}
