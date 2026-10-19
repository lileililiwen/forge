using System.Collections.Concurrent;
using System.Globalization;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle;

/// <summary>
/// In-memory <see cref="ITenantLifecycleStore"/> for development and test purposes.
/// Production code MUST register an application-owned durable store; the
/// in-memory store loses all state on restart and is not safe for multi-instance
/// orchestrators.
/// </summary>
public sealed class InMemoryTenantLifecycleStore : ITenantLifecycleStore
{
    private readonly ConcurrentDictionary<string, OperationEntry> _operations = new(StringComparer.Ordinal);
    private readonly object _gate = new();
    private long _sequence;

    /// <summary>The number of operations currently held in the store.</summary>
    public int OperationCount => _operations.Count;

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationId> CreateOperationAsync(
        TenantLifecycleWorkflowName workflowName,
        string tenantId,
        IReadOnlyDictionary<string, string> metadata,
        CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(tenantId))
            throw new ArgumentException("Tenant id is required.", nameof(tenantId));
        var idValue = Interlocked.Increment(ref _sequence).ToString(CultureInfo.InvariantCulture);
        var id = TenantLifecycleOperationId.Parse(idValue);
        var now = DateTimeOffset.UtcNow;
        var entry = new OperationEntry(
            id,
            workflowName,
            tenantId,
            TenantLifecycleOperationState.Pending,
            new List<TenantLifecycleStepStatus>(),
            metadata,
            now,
            now);
        _operations[idValue] = entry;
        return ValueTask.FromResult(id);
    }

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationStatus?> GetStatusAsync(TenantLifecycleOperationId operationId, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry))
            return ValueTask.FromResult<TenantLifecycleOperationStatus?>(null);
        return ValueTask.FromResult<TenantLifecycleOperationStatus?>(BuildStatus(entry));
    }

    /// <inheritdoc />
    public ValueTask RecordStepStatusAsync(TenantLifecycleOperationId operationId, TenantLifecycleStepStatus stepStatus, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry))
            throw new InvalidOperationException($"Operation '{operationId}' was not found.");
        lock (_gate)
        {
            entry.StepStatuses.Add(stepStatus);
            entry.UpdatedAt = DateTimeOffset.UtcNow;
        }
        return ValueTask.CompletedTask;
    }

    /// <inheritdoc />
    public ValueTask SetOperationStateAsync(TenantLifecycleOperationId operationId, TenantLifecycleOperationState state, string? safeMessage, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry))
            throw new InvalidOperationException($"Operation '{operationId}' was not found.");
        lock (_gate)
        {
            if (IsTerminal(entry.State) && state != entry.State)
                throw new InvalidOperationException($"Cannot transition from terminal state {entry.State} to {state}.");
            entry.State = state;
            entry.SafeFailureMessage = safeMessage;
            entry.UpdatedAt = DateTimeOffset.UtcNow;
        }
        return ValueTask.CompletedTask;
    }

    private static bool IsTerminal(TenantLifecycleOperationState state) =>
        state is TenantLifecycleOperationState.Succeeded
            or TenantLifecycleOperationState.PermanentlyFailed
            or TenantLifecycleOperationState.Canceled
            or TenantLifecycleOperationState.PolicyDenied;

    private static TenantLifecycleOperationStatus BuildStatus(OperationEntry entry) => new(
        entry.OperationId,
        entry.WorkflowName,
        entry.TenantId,
        entry.State,
        entry.StepStatuses.Count(s => s.Outcome == TenantLifecycleStepOutcome.Succeeded),
        entry.StepStatuses.ToArray(),
        entry.CreatedAt,
        entry.UpdatedAt,
        entry.SafeFailureMessage);

    private sealed class OperationEntry
    {
        public OperationEntry(
            TenantLifecycleOperationId operationId,
            TenantLifecycleWorkflowName workflowName,
            string tenantId,
            TenantLifecycleOperationState state,
            List<TenantLifecycleStepStatus> stepStatuses,
            IReadOnlyDictionary<string, string> metadata,
            DateTimeOffset createdAt,
            DateTimeOffset updatedAt)
        {
            OperationId = operationId;
            WorkflowName = workflowName;
            TenantId = tenantId;
            State = state;
            StepStatuses = stepStatuses;
            Metadata = metadata;
            CreatedAt = createdAt;
            UpdatedAt = updatedAt;
        }
        public TenantLifecycleOperationId OperationId { get; }
        public TenantLifecycleWorkflowName WorkflowName { get; }
        public string TenantId { get; }
        public TenantLifecycleOperationState State { get; set; }
        public List<TenantLifecycleStepStatus> StepStatuses { get; }
        public IReadOnlyDictionary<string, string> Metadata { get; }
        public DateTimeOffset CreatedAt { get; }
        public DateTimeOffset UpdatedAt { get; set; }
        public string? SafeFailureMessage { get; set; }
    }
}
