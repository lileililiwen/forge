using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenancy.Sample;

/// <summary>
/// Application-owned durable store. Production code persists this in the
/// application's own database; the in-memory dictionary here only proves
/// the contract.
/// </summary>
public sealed class SampleTenantLifecycleStore : ITenantLifecycleStore
{
    private readonly object _gate = new();
    private readonly Dictionary<TenantLifecycleOperationId, List<TenantLifecycleStepStatus>> _steps = new();
    private readonly Dictionary<TenantLifecycleOperationId, (TenantLifecycleWorkflowName Workflow, string Tenant, TenantLifecycleOperationState State)> _operations = new();

    /// <summary>Gets the number of recorded step statuses.</summary>
    public int RecordedSteps
    {
        get
        {
            lock (_gate)
            {
                return _steps.Values.Sum(list => list.Count);
            }
        }
    }

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationId> CreateOperationAsync(
        TenantLifecycleWorkflowName workflowName,
        string tenantId,
        IReadOnlyDictionary<string, string> metadata,
        CancellationToken cancellationToken = default)
    {
        var id = new TenantLifecycleOperationId(Guid.NewGuid().ToString("N"));
        lock (_gate)
        {
            _operations[id] = (workflowName, tenantId, TenantLifecycleOperationState.Pending);
            _steps[id] = new List<TenantLifecycleStepStatus>();
        }

        return ValueTask.FromResult(id);
    }

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationStatus?> GetStatusAsync(
        TenantLifecycleOperationId operationId,
        CancellationToken cancellationToken = default)
    {
        lock (_gate)
        {
            if (!_operations.TryGetValue(operationId, out var operation))
            {
                return ValueTask.FromResult<TenantLifecycleOperationStatus?>(null);
            }

            var statuses = _steps[operationId].ToList();
            return ValueTask.FromResult<TenantLifecycleOperationStatus?>(new TenantLifecycleOperationStatus(
                operationId,
                operation.Workflow,
                operation.Tenant,
                operation.State,
                statuses.Count(status => status.Outcome == TenantLifecycleStepOutcome.Succeeded),
                statuses,
                DateTimeOffset.UtcNow,
                DateTimeOffset.UtcNow,
                null));
        }
    }

    /// <inheritdoc />
    public ValueTask RecordStepStatusAsync(
        TenantLifecycleOperationId operationId,
        TenantLifecycleStepStatus stepStatus,
        CancellationToken cancellationToken = default)
    {
        lock (_gate)
        {
            _steps[operationId].Add(stepStatus);
        }

        return ValueTask.CompletedTask;
    }

    /// <inheritdoc />
    public ValueTask SetOperationStateAsync(
        TenantLifecycleOperationId operationId,
        TenantLifecycleOperationState state,
        string? safeMessage,
        CancellationToken cancellationToken = default)
    {
        lock (_gate)
        {
            var operation = _operations[operationId];
            _operations[operationId] = (operation.Workflow, operation.Tenant, state);
        }

        return ValueTask.CompletedTask;
    }
}
