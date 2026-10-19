namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>
/// Provider-neutral orchestrator for tenant lifecycle workflows. The orchestrator
/// runs steps in order, skips already-completed steps, classifies failures, and
/// records durable status through the registered <see cref="ITenantLifecycleStore"/>.
/// </summary>
public interface ITenantLifecycleOrchestrator
{
    /// <summary>Starts a new operation for <paramref name="tenantId"/> and runs the workflow to completion or terminal failure.</summary>
    ValueTask<TenantLifecycleOperationStatus> StartAsync(
        ITenantLifecycleWorkflow workflow,
        string tenantId,
        IReadOnlyDictionary<string, string> metadata,
        CancellationToken cancellationToken = default);

    /// <summary>Resumes an existing operation, skipping completed steps. Returns the latest status on success or failure.</summary>
    ValueTask<TenantLifecycleOperationStatus> ResumeAsync(
        TenantLifecycleOperationId operationId,
        CancellationToken cancellationToken = default);
}
