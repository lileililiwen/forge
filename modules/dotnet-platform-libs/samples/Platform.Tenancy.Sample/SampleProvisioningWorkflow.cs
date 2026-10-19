using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenancy.Sample;

/// <summary>
/// Application-owned provisioning workflow. Steps are idempotent by
/// contract and record the tenants they touched so tests can observe
/// ordering without a database.
/// </summary>
public sealed class SampleProvisioningWorkflow : ITenantLifecycleWorkflow
{
    /// <inheritdoc />
    public TenantLifecycleWorkflowName Name { get; } = TenantLifecycleNames.Workflow("sample-provisioning");

    /// <summary>Gets the tenants each step executed for, in execution order.</summary>
    public List<(string Step, string Tenant)> Executions { get; } = new();

    /// <inheritdoc />
    public IReadOnlyList<ITenantLifecycleStep> Steps { get; }

    /// <summary>Creates the workflow with its two application-owned steps.</summary>
    public SampleProvisioningWorkflow()
    {
        Steps = new List<ITenantLifecycleStep>
        {
            new RecordingStep(this, TenantLifecycleNames.Step("create-schema"), order: 1),
            new RecordingStep(this, TenantLifecycleNames.Step("seed-defaults"), order: 2),
        };
    }

    private sealed class RecordingStep : ITenantLifecycleStep
    {
        private readonly SampleProvisioningWorkflow _workflow;

        public RecordingStep(SampleProvisioningWorkflow workflow, TenantLifecycleStepName name, int order)
        {
            _workflow = workflow;
            Name = name;
            Order = order;
        }

        public TenantLifecycleStepName Name { get; }

        public int Order { get; }

        public bool IsTenantScoped => true;

        public ValueTask<TenantLifecycleStepResult> ExecuteAsync(TenantLifecycleStepContext context, CancellationToken cancellationToken = default)
        {
            _workflow.Executions.Add((Name.Value, context.TenantId));
            return ValueTask.FromResult(TenantLifecycleStepResult.Succeeded("sample step applied"));
        }
    }
}
