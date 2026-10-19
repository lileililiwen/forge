using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>
/// The final destination for an audit event. Applications own the concrete sink (database, queue,
/// file, or external service). The platform ships no persistence implementation.
/// </summary>
public interface IAuditSink
{
    /// <summary>Records a single, already-enriched and masked audit event.</summary>
    /// <param name="auditEvent">The event to record.</param>
    /// <param name="cancellationToken">A token that may cancel the operation.</param>
    /// <returns>A task completing when the sink has accepted the event.</returns>
    Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default);
}
