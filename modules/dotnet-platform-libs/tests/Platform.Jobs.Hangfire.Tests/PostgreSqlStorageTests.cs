using System.Diagnostics;
using Hangfire;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Platform.Jobs;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

namespace Platform.Jobs.Hangfire.Tests;

/// <summary>
/// Optional PostgreSQL integration test. The test is Docker-gated: when a
/// working Docker daemon is unavailable (or the PostgreSQL image cannot be
/// pulled), the test returns without asserting so the suite stays
/// deterministic on machines without Docker.
/// </summary>
public class PostgreSqlStorageTests : HangfireTest
{
    private const string Image = "postgres:16-alpine";

    [Fact]
    public async Task PostgreSql_storage_dispatches_and_reports_health_when_docker_is_available()
    {
        if (!TryStartPostgreSqlContainer(out var containerId, out var connectionString))
        {
            return;
        }

        try
        {
            var handler = new RecordingPayloadHandler();
            var builder = Host.CreateApplicationBuilder();
            builder.Services.AddLogging();
            builder.Services.AddPlatformHangfireJobs(options =>
            {
                options.Storage = HangfireStorageKind.PostgreSql;
                options.PostgreSqlConnectionString = connectionString;
                options.SchedulePollingInterval = TimeSpan.FromSeconds(1);
            });
            builder.Services.AddSingleton<IJobPayloadHandler>(handler);
            using var host = builder.Build();
            await host.StartAsync();
            try
            {
                var dispatcher = host.Services.GetRequiredService<IJobDispatcher>();
                await dispatcher.EnqueueAsync(JobPayload.Create("pg-cleanup", new Dictionary<string, object?> { ["source"] = "pg" }));

                var handled = await WaitAsync(handler.HandledTask);
                Assert.Equal("pg-cleanup", handled.Name);

                var check = host.Services.GetRequiredService<IHealthCheck>();
                var result = await check.CheckHealthAsync(null);
                Assert.Equal(HealthStatus.Healthy, result.Status);
            }
            finally
            {
                await host.StopAsync();
            }
        }
        finally
        {
            Docker("rm", "-f", containerId);
        }
    }

    private static bool TryStartPostgreSqlContainer(out string containerId, out string connectionString)
    {
        containerId = string.Empty;
        connectionString = string.Empty;

        if (Docker("info") is null)
        {
            return false;
        }

        var started = Docker("run", "-d", "--rm", "-e", "POSTGRES_PASSWORD=test", "-P", Image);
        if (started is null)
        {
            return false;
        }

        containerId = started.Trim();
        for (var attempt = 0; attempt < 60; attempt++)
        {
            var port = Docker("port", containerId, "5432");
            if (!string.IsNullOrWhiteSpace(port))
            {
                var parts = port.Trim().Split(':');
                var hostPort = parts[^1];
                connectionString = $"Host=localhost;Port={hostPort};Database=postgres;Username=postgres;Password=test";
                if (Docker("exec", containerId, "pg_isready", "-U", "postgres") is not null)
                {
                    return true;
                }
            }

            Thread.Sleep(TimeSpan.FromSeconds(1));
        }

        Docker("rm", "-f", containerId);
        return false;
    }

    private static string? Docker(params string[] arguments)
    {
        try
        {
            using var process = Process.Start(
                new ProcessStartInfo("docker", string.Join(' ', arguments))
                {
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                });
            if (process is null)
            {
                return null;
            }

            var output = process.StandardOutput.ReadToEnd();
            process.WaitForExit(TimeSpan.FromSeconds(60));
            return process.ExitCode == 0 ? output : null;
        }
        catch
        {
            return null;
        }
    }

    private static async Task<T> WaitAsync<T>(Task<T> task)
    {
        var completed = await Task.WhenAny(task, Task.Delay(TimeSpan.FromSeconds(30)));
        if (completed != task)
        {
            throw new TimeoutException("The PostgreSQL-backed job did not complete within 30 seconds.");
        }

        return await task;
    }
}
