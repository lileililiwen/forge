using Platform.Core.Context;

namespace Platform.Realtime.Tenant;

/// <summary>
/// Fail-closed default router. A message may be delivered only when its target
/// tenant equals the caller's tenant (or when neither carries a tenant). Cross
/// tenant and broadcast delivery are denied until an application registers its
/// own <see cref="IRealtimeTenantRouter"/>.
/// </summary>
public sealed class DenyCrossTenantRouter : IRealtimeTenantRouter
{
    /// <inheritdoc />
    public ValueTask<bool> IsRouteAllowedAsync(
        RealtimeTenantRoute route,
        CancellationToken cancellationToken = default)
    {
        var callerTenant = route.Caller.TenantId;
        var targetTenant = route.TargetTenantId;

        var allowed = string.Equals(callerTenant, targetTenant, StringComparison.Ordinal);
        return new ValueTask<bool>(allowed);
    }
}
