using Microsoft.Extensions.Diagnostics.HealthChecks;

namespace Platform.Persistence.EfCore.Migrations;

/// <summary>Exposes migration status as a readiness-only health check.</summary>
public sealed class EfCoreReadinessCheck(IMigrationStatusReader reader) : IHealthCheck
{
    /// <inheritdoc />
    public async Task<HealthCheckResult> CheckHealthAsync(HealthCheckContext context, CancellationToken cancellationToken = default)
    {
        var status = await reader.GetStatusAsync(cancellationToken);
        return status.IsReady
            ? HealthCheckResult.Healthy("Database is reachable and has no pending migrations.")
            : HealthCheckResult.Unhealthy(status.Error ?? $"Database has {status.PendingMigrationCount} pending migrations.");
    }
}
