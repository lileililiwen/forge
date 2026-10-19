namespace Platform.Tenant.Lifecycle.Contracts;

/// <summary>
/// Application-owned durable store for tenant lifecycle operations. The platform
/// provides only the contract; the application supplies an EF Core, document,
/// or otherwise persisted implementation.
/// </summary>
public interface ITenantLifecycleStore
{
    /// <summary>Creates a new operation record and returns the assigned <see cref="TenantLifecycleOperationId"/>.</summary>
    ValueTask<TenantLifecycleOperationId> CreateOperationAsync(
        TenantLifecycleWorkflowName workflowName,
        string tenantId,
        IReadOnlyDictionary<string, string> metadata,
        CancellationToken cancellationToken = default);

    /// <summary>Returns the latest status for <paramref name="operationId"/>, or <c>null</c> when no such operation exists.</summary>
    ValueTask<TenantLifecycleOperationStatus?> GetStatusAsync(
        TenantLifecycleOperationId operationId,
        CancellationToken cancellationToken = default);

    /// <summary>Appends a step status to the operation's history.</summary>
    ValueTask RecordStepStatusAsync(
        TenantLifecycleOperationId operationId,
        TenantLifecycleStepStatus stepStatus,
        CancellationToken cancellationToken = default);

    /// <summary>Updates the operation's overall state. The store MUST reject transitions from terminal states.</summary>
    ValueTask SetOperationStateAsync(
        TenantLifecycleOperationId operationId,
        TenantLifecycleOperationState state,
        string? safeMessage,
        CancellationToken cancellationToken = default);
}
