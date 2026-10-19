using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>
/// Receives events that a sink failed to accept under the fail-open policy, so they are not
/// silently lost. The platform owns no durable store; applications provide a concrete sink.
/// </summary>
public interface IAuditDeadLetterSink
{
    /// <summary>Records a failed event and the reason it was not delivered.</summary>
    /// <param name="auditEvent">The event that could not be delivered.</param>
    /// <param name="failure">The safe failure detail. Must not contain sink secrets.</param>
    /// <param name="cancellationToken">A token that may cancel the operation.</param>
    /// <returns>A task completing when the event has been recorded for later inspection.</returns>
    Task RecordFailedAsync(AuditEvent auditEvent, string failure, CancellationToken cancellationToken = default);
}
