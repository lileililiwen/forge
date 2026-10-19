using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>The default dead-letter sink that discards failed events. Applications replace it to capture delivery failures.</summary>
public sealed class NoOpAuditDeadLetterSink : IAuditDeadLetterSink
{
    /// <inheritdoc />
    public Task RecordFailedAsync(AuditEvent auditEvent, string failure, CancellationToken cancellationToken = default)
        => Task.CompletedTask;
}
