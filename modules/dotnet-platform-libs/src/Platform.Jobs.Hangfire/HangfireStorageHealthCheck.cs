using Hangfire;
using Hangfire.Storage;
using Microsoft.Extensions.Diagnostics.HealthChecks;

namespace Platform.Jobs.Hangfire;

/// <summary>
/// Health check that probes Hangfire storage reachability. A successful
/// probe reports healthy; a failed probe reports unhealthy with a
/// redacted diagnostic that carries only the exception type name — never
/// the exception message, connection string, or storage response.
/// </summary>
public sealed class HangfireStorageHealthCheck : IHealthCheck
{
    /// <summary>Stable error code reported when the storage probe fails.</summary>
    public const string StorageUnavailableCode = "jobs.storage.unavailable";

    private readonly JobStorage _storage;
    private volatile HangfireJobsProviderStatus _status =
        new(HangfireJobsProviderStatus.ProviderName, HangfireJobsProviderState.Unavailable, StorageUnavailableCode);

    /// <summary>
    /// Initializes a new instance of the <see cref="HangfireStorageHealthCheck"/> class.
    /// </summary>
    /// <param name="storage">The Hangfire storage the adapter registered.</param>
    public HangfireStorageHealthCheck(JobStorage storage)
    {
        ArgumentNullException.ThrowIfNull(storage);
        _storage = storage;
    }

    /// <summary>
    /// Gets the documented snapshot of the most recent probe outcome. The
    /// snapshot never carries connection strings or credentials.
    /// </summary>
    public HangfireJobsProviderStatus Status => _status;

    /// <inheritdoc />
    public Task<HealthCheckResult> CheckHealthAsync(HealthCheckContext? context, CancellationToken cancellationToken = default)
    {
        try
        {
            _ = _storage.GetMonitoringApi().Queues();
            _status = new HangfireJobsProviderStatus(HangfireJobsProviderStatus.ProviderName, HangfireJobsProviderState.Healthy);
            return Task.FromResult(HealthCheckResult.Healthy("Hangfire storage is reachable."));
        }
#pragma warning disable CA1031 // Health checks must catch all exceptions to report degraded/unhealthy
        catch (Exception ex)
        {
            _status = new HangfireJobsProviderStatus(
                HangfireJobsProviderStatus.ProviderName,
                HangfireJobsProviderState.Unavailable,
                StorageUnavailableCode);
            return Task.FromResult(HealthCheckResult.Unhealthy(
                "Hangfire storage is unavailable.",
                null,
                new Dictionary<string, object>
                {
                    ["provider"] = HangfireJobsProviderStatus.ProviderName,
                    ["error"] = StorageUnavailableCode,
                    ["exceptionType"] = ex.GetType().Name,
                }));
        }
#pragma warning restore CA1031
    }
}
