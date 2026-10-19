namespace Platform.Realtime.Tenant;

/// <summary>
/// Application-owned decision point invoked before a message is delivered to a
/// tenant scope. The platform never broadcasts cross-tenant data without a
/// positive result from this contract.
/// </summary>
public interface IRealtimeTenantRouter
{
    /// <summary>
    /// Returns whether the <paramref name="route"/> may be delivered given the
    /// caller and the target tenant.
    /// </summary>
    /// <param name="route">The candidate delivery route.</param>
    /// <param name="cancellationToken">A token that cancels the routing check.</param>
    /// <returns><c>true</c> when delivery is permitted; otherwise <c>false</c>.</returns>
    ValueTask<bool> IsRouteAllowedAsync(
        RealtimeTenantRoute route,
        CancellationToken cancellationToken = default);
}
