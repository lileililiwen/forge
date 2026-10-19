using System.Collections.Concurrent;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle.Testing;

/// <summary>
/// In-memory <see cref="ITenantLifecycleStore"/> for tests. Deterministic
/// in-process state, no concurrency primitives shared across instances.
/// </summary>
public sealed class InMemoryTenantLifecycleStore : ITenantLifecycleStore
{
    private readonly ConcurrentDictionary<string, Entry> _operations = new(StringComparer.Ordinal);
    private long _sequence;

    /// <summary>The number of operations held in the store.</summary>
    public int Count => _operations.Count;

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationId> CreateOperationAsync(TenantLifecycleWorkflowName workflowName, string tenantId, IReadOnlyDictionary<string, string> metadata, CancellationToken cancellationToken = default)
    {
        if (string.IsNullOrWhiteSpace(tenantId)) throw new ArgumentException("Tenant id is required.", nameof(tenantId));
        var idValue = Interlocked.Increment(ref _sequence).ToString(System.Globalization.CultureInfo.InvariantCulture);
        var id = TenantLifecycleOperationId.Parse(idValue);
        var now = DateTimeOffset.UtcNow;
        _operations[idValue] = new Entry(id, workflowName, tenantId, TenantLifecycleOperationState.Pending, new List<TenantLifecycleStepStatus>(), metadata, now, now);
        return ValueTask.FromResult(id);
    }

    /// <inheritdoc />
    public ValueTask<TenantLifecycleOperationStatus?> GetStatusAsync(TenantLifecycleOperationId operationId, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry)) return ValueTask.FromResult<TenantLifecycleOperationStatus?>(null);
        return ValueTask.FromResult<TenantLifecycleOperationStatus?>(Build(entry));
    }

    /// <inheritdoc />
    public ValueTask RecordStepStatusAsync(TenantLifecycleOperationId operationId, TenantLifecycleStepStatus stepStatus, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry)) throw new InvalidOperationException($"Operation '{operationId}' was not found.");
        lock (entry.Gate)
        {
            entry.StepStatuses.Add(stepStatus);
            entry.UpdatedAt = DateTimeOffset.UtcNow;
        }
        return ValueTask.CompletedTask;
    }

    /// <inheritdoc />
    public ValueTask SetOperationStateAsync(TenantLifecycleOperationId operationId, TenantLifecycleOperationState state, string? safeMessage, CancellationToken cancellationToken = default)
    {
        if (!_operations.TryGetValue(operationId.Value, out var entry)) throw new InvalidOperationException($"Operation '{operationId}' was not found.");
        lock (entry.Gate)
        {
            entry.State = state;
            entry.SafeFailureMessage = safeMessage;
            entry.UpdatedAt = DateTimeOffset.UtcNow;
        }
        return ValueTask.CompletedTask;
    }

    private static TenantLifecycleOperationStatus Build(Entry entry) => new(
        entry.OperationId, entry.WorkflowName, entry.TenantId, entry.State,
        entry.StepStatuses.Count(s => s.Outcome == TenantLifecycleStepOutcome.Succeeded),
        entry.StepStatuses.ToArray(), entry.CreatedAt, entry.UpdatedAt, entry.SafeFailureMessage);

    private sealed class Entry
    {
        public Entry(TenantLifecycleOperationId id, TenantLifecycleWorkflowName workflow, string tenant, TenantLifecycleOperationState state, List<TenantLifecycleStepStatus> steps, IReadOnlyDictionary<string, string> metadata, DateTimeOffset createdAt, DateTimeOffset updatedAt)
        {
            OperationId = id; WorkflowName = workflow; TenantId = tenant; State = state; StepStatuses = steps; Metadata = metadata; CreatedAt = createdAt; UpdatedAt = updatedAt;
        }
        public object Gate { get; } = new();
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
