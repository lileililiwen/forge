using Platform.Tenant.Lifecycle;
using Platform.Tenant.Lifecycle.Contracts;
using Platform.Tenant.Lifecycle.Testing;

namespace Platform.Tenant.Lifecycle.Tests;

public sealed class TenantLifecycleOrchestratorTests
{
    [Fact]
    public async Task StartAsync_runs_steps_in_order_and_records_succeeded()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepB = new ScriptedLifecycleStep(TenantLifecycleNames.Step("b"), 2, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA, stepB });

        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        Assert.Equal(TenantLifecycleOperationState.Succeeded, status.State);
        Assert.Equal(2, status.CompletedStepCount);
        Assert.Equal(2, scope.TenantIds.Count);
    }

    [Fact]
    public async Task StartAsync_marks_retryable_when_a_step_classifies_as_retryable()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepB = new ScriptedLifecycleStep(TenantLifecycleNames.Step("b"), 2, isTenantScoped: true, TenantLifecycleStepResult.Retryable("upstream 503"));
        var stepC = new ScriptedLifecycleStep(TenantLifecycleNames.Step("c"), 3, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA, stepB, stepC });

        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        Assert.Equal(TenantLifecycleOperationState.Retryable, status.State);
        Assert.Equal("upstream 503", status.SafeFailureMessage);
        Assert.Empty(stepC.Invocations);
    }

    [Fact]
    public async Task ResumeAsync_skips_completed_steps_and_recovers()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepB = new ScriptedLifecycleStep(TenantLifecycleNames.Step("b"), 2, isTenantScoped: true, TenantLifecycleStepResult.Retryable("transient"));
        var stepC = new ScriptedLifecycleStep(TenantLifecycleNames.Step("c"), 3, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA, stepB, stepC });
        var registry = new TenantLifecycleWorkflowRegistry().Register(workflow);
        orchestrator.WithWorkflowRegistry(registry);

        var started = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);
        Assert.Equal(TenantLifecycleOperationState.Retryable, started.State);

        stepB.Enqueue(TenantLifecycleStepResult.Succeeded("ok"));
        var resumed = await orchestrator.ResumeAsync(started.OperationId, CancellationToken.None);

        Assert.Equal(TenantLifecycleOperationState.Succeeded, resumed.State);
        Assert.Single(stepA.Invocations);
        Assert.Equal(2, stepB.Invocations.Count);
        Assert.Single(stepC.Invocations);
    }

    [Fact]
    public async Task StartAsync_marks_permanently_failed_when_a_step_classifies_as_permanent()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepB = new ScriptedLifecycleStep(TenantLifecycleNames.Step("b"), 2, isTenantScoped: true, TenantLifecycleStepResult.Permanent("invalid config"));
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA, stepB });

        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        Assert.Equal(TenantLifecycleOperationState.PermanentlyFailed, status.State);
        Assert.Equal("invalid config", status.SafeFailureMessage);
    }

    [Fact]
    public async Task StartAsync_marks_canceled_when_token_is_cancelled()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        using var cts = new CancellationTokenSource();
        var stepA = new DelegateLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, async (_, ct) =>
        {
            await Task.Delay(50, ct);
            return TenantLifecycleStepResult.Succeeded();
        });
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        cts.CancelAfter(5);

        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), cts.Token);

        Assert.Equal(TenantLifecycleOperationState.Canceled, status.State);
    }

    [Fact]
    public async Task Tenant_scoped_steps_install_and_dispose_the_tenant_scope_around_each_step()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepB = new ScriptedLifecycleStep(TenantLifecycleNames.Step("b"), 2, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA, stepB });

        await orchestrator.StartAsync(workflow, "tenant-9", new Dictionary<string, string>(), CancellationToken.None);

        Assert.Equal(0, scope.ActiveScopes);
    }

    [Fact]
    public async Task Duplicate_step_names_in_a_workflow_are_rejected()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA1 = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var stepA2 = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 2, isTenantScoped: true, TenantLifecycleStepResult.Succeeded());
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA1, stepA2 });

        var exception = await Assert.ThrowsAsync<ArgumentException>(() => orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None).AsTask());
        Assert.Contains("more than once", exception.Message);
    }

    [Fact]
    public async Task ResumeAsync_throws_when_operation_is_unknown()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        await Assert.ThrowsAsync<InvalidOperationException>(() => orchestrator.ResumeAsync(TenantLifecycleOperationId.Parse("missing"), CancellationToken.None).AsTask());
    }

    [Fact]
    public async Task Safe_message_is_preserved_on_step_status_records()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Retryable("upstream timeout"));
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });

        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);

        Assert.Single(status.StepStatuses);
        Assert.Equal(TenantLifecycleStepOutcome.Retryable, status.StepStatuses[0].Outcome);
        Assert.Equal("upstream timeout", status.StepStatuses[0].SafeMessage);
    }

    [Fact]
    public async Task ResumeAsync_throws_when_workflow_is_not_registered()
    {
        var store = new InMemoryTenantLifecycleStore();
        var scope = new RecordingLifecycleScopeCallback();
        var orchestrator = new TenantLifecycleOrchestrator(store, scope);
        var stepA = new ScriptedLifecycleStep(TenantLifecycleNames.Step("a"), 1, isTenantScoped: true, TenantLifecycleStepResult.Retryable("transient"));
        var workflow = new StaticLifecycleWorkflow(TenantLifecycleNames.Workflow("onboard"), new ITenantLifecycleStep[] { stepA });
        var status = await orchestrator.StartAsync(workflow, "tenant-1", new Dictionary<string, string>(), CancellationToken.None);
        Assert.Equal(TenantLifecycleOperationState.Retryable, status.State);

        await Assert.ThrowsAsync<InvalidOperationException>(() => orchestrator.ResumeAsync(status.OperationId, CancellationToken.None).AsTask());
    }
}
