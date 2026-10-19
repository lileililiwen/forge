using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>
/// A thread-safe in-memory dead-letter sink used for tests and local development. It records the
/// failed event alongside the safe failure reason so delivery gaps are observable without a store.
/// </summary>
public sealed class InMemoryAuditDeadLetterSink : IAuditDeadLetterSink
{
    private readonly ConcurrentQueue<(AuditEvent Event, string Failure)> _failed = new();

    /// <inheritdoc />
    public Task RecordFailedAsync(AuditEvent auditEvent, string failure, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(auditEvent);
        _failed.Enqueue((auditEvent, failure ?? string.Empty));
        return Task.CompletedTask;
    }

    /// <summary>Returns a snapshot of the dead-lettered events in insertion order.</summary>
    /// <returns>The dead-lettered events with their failure reasons.</returns>
    public IReadOnlyList<(AuditEvent Event, string Failure)> GetFailed() => _failed.ToArray();
}
