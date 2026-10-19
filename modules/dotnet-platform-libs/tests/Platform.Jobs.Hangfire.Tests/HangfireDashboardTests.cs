using Hangfire;
using Hangfire.Dashboard;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Jobs.Hangfire;
using Platform.Jobs.Hangfire.DependencyInjection;

namespace Platform.Jobs.Hangfire.Tests;

public class HangfireDashboardTests : HangfireTest
{
    [Fact]
    public void Factory_hides_the_storage_connection_string_and_installs_the_callback_filter()
    {
        var options = HangfireDashboardOptionsFactory.Create(_ => true);

        Assert.False(options.DisplayStorageConnectionString);
        Assert.Single(options.Authorization!.Cast<CallbackDashboardAuthorizationFilter>());
    }

    [Fact]
    public void Factory_rejects_null_callbacks()
    {
        Assert.Throws<ArgumentNullException>(() => HangfireDashboardOptionsFactory.Create(null!));
    }

    [Fact]
    public void Mapping_is_a_no_op_when_the_dashboard_is_disabled()
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformHangfireJobs();
        var app = builder.Build();

        Assert.Same(app, app.UsePlatformHangfireDashboard());
    }

    [Fact]
    public void Mapping_fails_fast_when_the_dashboard_is_enabled_without_a_callback()
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        var app = builder.Build();

        Assert.Throws<InvalidOperationException>(
            () => app.UsePlatformHangfireDashboard(new HangfireJobsOptions { DashboardEnabled = true }));
    }

    [Fact]
    public void Unauthorized_dashboard_requests_are_denied()
    {
        using var app = BuildDashboardApp(_ => false);
        var client = app.GetTestClient();

        var response = client.GetAsync("/jobs").GetAwaiter().GetResult();

        Assert.Equal(401, (int)response.StatusCode);
    }

    [Fact]
    public void Authorized_dashboard_requests_are_served_without_storage_details()
    {
        using var app = BuildDashboardApp(_ => true);
        var client = app.GetTestClient();

        var response = client.GetAsync("/jobs").GetAwaiter().GetResult();
        var body = response.Content.ReadAsStringAsync().GetAwaiter().GetResult();

        Assert.Equal(200, (int)response.StatusCode);
        Assert.DoesNotContain("InMemory", body, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Registration_rejects_an_enabled_dashboard_without_a_callback()
    {
        var services = new ServiceCollection();

        Assert.Throws<ArgumentException>(
            () => services.AddPlatformHangfireJobs(options => options.DashboardEnabled = true));
    }

    private static WebApplication BuildDashboardApp(Func<DashboardContext, bool> authorization)
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformHangfireJobs(options =>
        {
            options.DashboardEnabled = true;
            options.DashboardAuthorization = authorization;
        });
        var app = builder.Build();
        app.UsePlatformHangfireDashboard();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
