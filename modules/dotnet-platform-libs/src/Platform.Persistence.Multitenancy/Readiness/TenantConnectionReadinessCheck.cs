using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Core.Tenancy;

namespace Platform.Persistence.Multitenancy.Readiness;

/// <summary>
/// Probes the connection for every resolved tenant and aggregates the
/// outcomes into a single readiness check. The check is registered
/// with the <c>ready</c> tag so the platform readiness endpoint can
/// filter on it.
/// </summary>
public sealed class TenantConnectionReadinessCheck : IHealthCheck
{
    private static readonly string[] ReadyTag = { "ready" };

    private readonly ITenantInfo[] _tenants;
    private readonly ITenantConnectionReadinessProbe _probe;

    /// <summary>Creates a new check over the supplied tenant list.</summary>
    public TenantConnectionReadinessCheck(
        IEnumerable<ITenantInfo> tenants,
        ITenantConnectionReadinessProbe probe)
    {
        ArgumentNullException.ThrowIfNull(tenants);
        ArgumentNullException.ThrowIfNull(probe);
        _tenants = tenants.ToArray();
        _probe = probe;
    }

    /// <summary>Tag used when registering the check.</summary>
    public static IEnumerable<string> Tag => ReadyTag;

    /// <inheritdoc />
    public async Task<HealthCheckResult> CheckHealthAsync(
        HealthCheckContext context,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(context);
        if (_tenants.Length == 0)
        {
            return HealthCheckResult.Healthy("No tenant connections to probe.");
        }
        var failed = new List<string>();
        foreach (var tenant in _tenants)
        {
            var result = await _probe.ProbeAsync(tenant, cancellationToken).ConfigureAwait(false);
            if (!result.IsReady)
            {
                failed.Add(tenant.Id);
            }
        }
        if (failed.Count == 0)
        {
            return HealthCheckResult.Healthy("All tenant connections are reachable.");
        }
        return HealthCheckResult.Unhealthy(
            $"Tenant connections unavailable: {string.Join(", ", failed)}");
    }
}
