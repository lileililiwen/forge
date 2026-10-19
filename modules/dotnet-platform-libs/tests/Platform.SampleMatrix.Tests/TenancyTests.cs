using Microsoft.Extensions.DependencyInjection;
using Platform.Tenancy.Sample;
using Platform.Tenant.Lifecycle;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.SampleMatrix.Tests;

/// <summary>Stage 4: the application-owned workflow runs to completion through the orchestrator.</summary>
public sealed class TenancyTests
{
    /// <summary>Verifies ordered step execution with durable status.</summary>
    [Fact]
    public async Task Provisioning_runs_steps_in_order()
    {
        var store = new SampleTenantLifecycleStore();
        var workflow = new SampleProvisioningWorkflow();
        var services = new ServiceCollection();
        services.AddSingleton<ITenantLifecycleStore>(store);
        services.AddPlatformTenantLifecycle();
        await using var provider = services.BuildServiceProvider();
        provider.GetRequiredService<TenantLifecycleWorkflowRegistry>().Register(workflow);
        var orchestrator = provider.GetRequiredService<ITenantLifecycleOrchestrator>();

        var status = await orchestrator.StartAsync(
            workflow,
            "tenant-matrix-test",
            new Dictionary<string, string>());

        Assert.Equal(TenantLifecycleOperationState.Succeeded, status.State);
        Assert.Equal(2, status.CompletedStepCount);
        Assert.Equal(
            new[] { ("create-schema", "tenant-matrix-test"), ("seed-defaults", "tenant-matrix-test") },
            workflow.Executions.ToArray());
        Assert.Equal(2, store.RecordedSteps);
    }
}
