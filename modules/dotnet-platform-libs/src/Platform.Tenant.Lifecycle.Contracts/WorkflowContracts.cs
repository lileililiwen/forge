namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>
/// Application-owned tenant lifecycle step. The platform never implements this interface;
/// the application supplies the step and tells the platform how to install the tenant
/// scope around it.
/// </summary>
public interface ITenantLifecycleStep
{
    /// <summary>Stable name used as the idempotency boundary. MUST be unique within the workflow.</summary>
    TenantLifecycleStepName Name { get; }

    /// <summary>Returns the step's stable execution order. Lower values run first; ties resolve by <see cref="Name"/>.</summary>
    int Order { get; }

    /// <summary>Gets a value indicating whether the step runs under a tenant scope installed by the orchestrator.</summary>
    bool IsTenantScoped { get; }

    /// <summary>Runs the step. MUST be idempotent: a retry with the same <paramref name="context"/> produces the same side effects.</summary>
    /// <param name="context">The step context supplied by the orchestrator.</param>
    /// <param name="cancellationToken">Cancellation token observed by the step.</param>
    ValueTask<TenantLifecycleStepResult> ExecuteAsync(TenantLifecycleStepContext context, CancellationToken cancellationToken = default);
}

/// <summary>
/// Ordered, named tenant lifecycle workflow. The workflow is the operation identity:
/// two operations of the same workflow for the same tenant are resumable by replaying
/// the same <see cref="TenantLifecycleOperationId"/>.
/// </summary>
public interface ITenantLifecycleWorkflow
{
    /// <summary>Stable workflow name.</summary>
    TenantLifecycleWorkflowName Name { get; }
    /// <summary>The ordered steps. The workflow is invalid if any two steps share a name.</summary>
    IReadOnlyList<ITenantLifecycleStep> Steps { get; }
}

/// <summary>Application-owned callback the orchestrator invokes to install and restore tenant scope for tenant-scoped steps.</summary>
public interface ITenantLifecycleScopeCallback
{
    /// <summary>Installs a tenant scope for <paramref name="tenantId"/>. The returned disposable MUST restore the prior scope on dispose.</summary>
    IDisposable BeginTenantScope(string tenantId);
}
