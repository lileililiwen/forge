using Hangfire.InMemory;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Jobs.Hangfire;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireStorageHealthCheckTests : HangfireTest
{
    [Fact]
    public async Task Reachable_storage_reports_healthy()
    {
        var storage = new InMemoryStorage();
        var check = new HangfireStorageHealthCheck(storage);

        var result = await check.CheckHealthAsync(null);

        Assert.Equal(HealthStatus.Healthy, result.Status);
        Assert.Equal(HangfireJobsProviderState.Healthy, check.Status.State);
        Assert.Null(check.Status.LastErrorCode);
        Assert.Equal("hangfire", check.Status.Provider);
    }

    [Fact]
    public async Task Unreachable_storage_reports_unhealthy_with_a_redacted_diagnostic()
    {
        var check = new HangfireStorageHealthCheck(new ThrowingStorage());

        var result = await check.CheckHealthAsync(null);

        Assert.Equal(HealthStatus.Unhealthy, result.Status);
        Assert.Equal(HangfireJobsProviderState.Unavailable, check.Status.State);
        Assert.Equal(HangfireStorageHealthCheck.StorageUnavailableCode, check.Status.LastErrorCode);
        Assert.Equal("InvalidOperationException", result.Data["exceptionType"]);
        Assert.DoesNotContain("unreachable", result.Description, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("storage unreachable", string.Join(",", result.Data.Values.Select(v => v?.ToString() ?? string.Empty)), StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Status_starts_unavailable_before_the_first_probe()
    {
        var check = new HangfireStorageHealthCheck(new InMemoryStorage());

        Assert.Equal(HangfireJobsProviderState.Unavailable, check.Status.State);
        Assert.Equal(HangfireStorageHealthCheck.StorageUnavailableCode, check.Status.LastErrorCode);
    }
}
