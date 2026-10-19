namespace Platform.Core.Tenancy;

/// <summary>
/// Reports the readiness of the connection associated with a tenant.
/// Implementations must never throw; they return a stable outcome
/// that downstream health checks can project to liveness/readiness.
/// </summary>
public interface ITenantConnectionReadinessProbe
{
    /// <summary>Probes the connection associated with the supplied tenant, if any.</summary>
    /// <param name="tenant">The resolved tenant, or <c>null</c> to probe the shared connection.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    ValueTask<TenantConnectionReadinessResult> ProbeAsync(ITenantInfo? tenant, CancellationToken cancellationToken = default);
}

/// <summary>Outcome of a tenant connection probe.</summary>
/// <param name="IsReady">Whether the connection is reachable.</param>
/// <param name="ConnectionName">The descriptor name the probe targeted.</param>
/// <param name="Error">A safe, non-sensitive reason when the probe failed.</param>
public readonly record struct TenantConnectionReadinessResult(
    bool IsReady,
    string ConnectionName,
    string? Error)
{
    /// <summary>Builds a successful probe outcome.</summary>
    public static TenantConnectionReadinessResult Ready(string name) => new(true, name, null);

    /// <summary>Builds a failed probe outcome with a safe reason.</summary>
    public static TenantConnectionReadinessResult Failed(string name, string error) => new(false, name, error);
}
