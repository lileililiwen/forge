using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Jobs.DependencyInjection;

namespace Platform.Jobs.Tests;

public class TestServerIntegrationTests
{
    [Fact]
    public async Task Host_with_AddPlatformJobs_resolves_clock_and_default_options()
    {
        await using var app = BuildApp();
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"timeZone\":\"UTC\"", body);
        Assert.Contains("\"clock\":\"", body);
    }

    [Fact]
    public async Task Host_with_configured_section_overrides_defaults()
    {
        await using var app = BuildApp(extraConfiguration: new Dictionary<string, string?>
        {
            ["BackgroundJobs:DefaultTimeZone"] = "Europe/Berlin",
        });
        var client = app.GetTestClient();

        var response = await client.GetAsync("/probe");

        Assert.Equal(StatusCodes.Status200OK, (int)response.StatusCode);
        var body = await response.Content.ReadAsStringAsync();
        Assert.Contains("\"timeZone\":\"Europe/Berlin\"", body);
    }

    [Fact]
    public void AddPlatformJobs_does_not_register_default_JobDispatcher_or_Registry_or_Telemetry()
    {
        var services = new ServiceCollection();

        services.AddPlatformJobs();

        using var provider = services.BuildServiceProvider();
        Assert.Null(provider.GetService<IJobDispatcher>());
        Assert.Null(provider.GetService<IRecurringJobRegistry>());
        Assert.Null(provider.GetService<IJobTelemetry>());
    }

    private static WebApplication BuildApp(
        IReadOnlyDictionary<string, string?>? extraConfiguration = null)
    {
        var options = new WebApplicationOptions
        {
            EnvironmentName = "Testing",
        };
        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseTestServer();
        if (extraConfiguration is not null)
        {
            builder.Configuration.AddInMemoryCollection(extraConfiguration);
        }

        builder.Services.AddPlatformJobs(platformOptions =>
            builder.Configuration
                .GetSection(BackgroundJobsOptions.SectionName)
                .Bind(platformOptions));

        var app = builder.Build();
        app.MapGet("/probe", (IOptions<BackgroundJobsOptions> options, IClock clock) => new
        {
            timeZone = options.Value.DefaultTimeZone,
            clock = clock.UtcNow.ToString("O"),
        });

        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
