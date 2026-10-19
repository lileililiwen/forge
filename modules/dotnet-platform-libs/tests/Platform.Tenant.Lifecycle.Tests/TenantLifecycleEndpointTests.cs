using System.Net;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Tenant.Lifecycle;
using Platform.Tenant.Lifecycle.AspNetCore;
using Platform.Tenant.Lifecycle.Contracts;
using Platform.Tenant.Lifecycle.Testing;

namespace Platform.Tenant.Lifecycle.Tests;

public sealed class TenantLifecycleEndpointTests
{
    [Fact]
    public async Task Status_endpoint_returns_404_for_unknown_operation()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddPlatformTenantLifecycle();
            services.AddPlatformTenantLifecycleReadiness();
        });
        var client = app.GetTestClient();
        var response = await client.GetAsync("/platform/tenant-lifecycle/operations/9999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Status_endpoint_returns_succeeded_status()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddPlatformTenantLifecycle();
            services.AddPlatformTenantLifecycleReadiness();
        });
        var client = app.GetTestClient();
        var orchestrator = app.Services.GetRequiredService<ITenantLifecycleOrchestrator>();
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: false, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        var response = await client.GetAsync($"/platform/tenant-lifecycle/operations/{status.OperationId.Value}");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var snapshot = await response.Content.ReadFromJsonAsync<TenantLifecycleOperationStatus>();
        Assert.NotNull(snapshot);
        Assert.Equal(TenantLifecycleOperationState.Succeeded, snapshot!.State);
    }

    [Fact]
    public async Task Resume_endpoint_runs_a_retryable_operation_to_completion()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddPlatformTenantLifecycle();
            services.AddPlatformTenantLifecycleReadiness();
        });
        var client = app.GetTestClient();
        var orchestrator = app.Services.GetRequiredService<TenantLifecycleOrchestrator>();
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: false, TenantLifecycleStepResult.Retryable("transient"));
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        orchestrator.WithWorkflowRegistry(new TenantLifecycleWorkflowRegistry().Register(workflow));
        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);
        Assert.Equal(TenantLifecycleOperationState.Retryable, status.State);

        stepA.Enqueue(TenantLifecycleStepResult.Succeeded());
        var response = await client.PostAsync($"/platform/tenant-lifecycle/operations/{status.OperationId.Value}/resume", content: null);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var snapshot = await response.Content.ReadFromJsonAsync<TenantLifecycleOperationStatus>();
        Assert.NotNull(snapshot);
        Assert.Equal(TenantLifecycleOperationState.Succeeded, snapshot!.State);
    }

    [Fact]
    public async Task Readiness_check_reports_healthy_for_a_succeeded_operation()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddPlatformTenantLifecycle();
            services.AddPlatformTenantLifecycleReadiness();
        });
        var check = app.Services.GetServices<IReadinessCheck>().OfType<TenantLifecycleReadinessCheck>().Single();
        var orchestrator = app.Services.GetRequiredService<ITenantLifecycleOrchestrator>();
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: false, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        var result = await check.CheckAsync(new ReadinessContext(status.OperationId.Value), CancellationToken.None);
        Assert.True(result.IsHealthy);
        Assert.Equal(TenantLifecycleReasons.Ready, result.Reason);
    }

    [Fact]
    public async Task Readiness_check_reports_unhealthy_for_a_retryable_operation()
    {
        await using var app = BuildApp(services: services =>
        {
            services.AddPlatformTenantLifecycle();
            services.AddPlatformTenantLifecycleReadiness();
        });
        var check = app.Services.GetServices<IReadinessCheck>().OfType<TenantLifecycleReadinessCheck>().Single();
        var orchestrator = app.Services.GetRequiredService<ITenantLifecycleOrchestrator>();
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: false, TenantLifecycleStepResult.Retryable("upstream 503"));
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        var result = await check.CheckAsync(new ReadinessContext(status.OperationId.Value), CancellationToken.None);
        Assert.False(result.IsHealthy);
        Assert.Equal(TenantLifecycleReasons.Retryable, result.Reason);
        Assert.Equal("upstream 503", result.Detail);
    }

    private static WebApplication BuildApp(Action<IServiceCollection>? services = null)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        services?.Invoke(builder.Services);
        var app = builder.Build();
        app.MapPlatformTenantLifecycleStatus();
        app.MapPlatformTenantLifecycleResume();
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
