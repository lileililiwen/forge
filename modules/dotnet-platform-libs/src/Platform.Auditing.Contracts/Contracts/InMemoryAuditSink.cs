using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>
/// A bounded, thread-safe in-memory sink used as the default registration and for tests. It does
/// not persist events; applications replace it with a durable sink. Capacity bounds the retained
/// event count, evicting the oldest recorded event when exceeded.
/// </summary>
public sealed class InMemoryAuditSink : IAuditSink
{
    private readonly ConcurrentQueue<AuditEvent> _events = new();
    private readonly int _capacity;
    private long _accepted;

    /// <summary>Initializes a new sink with the supplied capacity.</summary>
    /// <param name="capacity">The maximum number of retained events. Must be positive.</param>
    public InMemoryAuditSink(int capacity = 1024)
    {
        if (capacity <= 0) throw new ArgumentOutOfRangeException(nameof(capacity), capacity, "Capacity must be positive.");
        _capacity = capacity;
    }

    /// <inheritdoc />
    public Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(auditEvent);
        Interlocked.Increment(ref _accepted);
        _events.Enqueue(auditEvent);
        while (_events.Count > _capacity && _events.TryDequeue(out _))
        {
        }

        return Task.CompletedTask;
    }

    /// <summary>Returns a snapshot of the recorded events in insertion order.</summary>
    /// <returns>The recorded events.</returns>
    public IReadOnlyList<AuditEvent> GetRecorded() => _events.ToArray();

    /// <summary>The total number of events accepted by this sink, including any evicted.</summary>
    public long AcceptedCount => Interlocked.Read(ref _accepted);
}
