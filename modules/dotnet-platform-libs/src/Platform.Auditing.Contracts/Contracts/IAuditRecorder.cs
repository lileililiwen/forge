using System.Threading;
using System.Threading.Tasks;

namespace Platform.Auditing.Contracts;

/// <summary>
/// The entry point used by capture adapters (HTTP, exception, EF). The recorder enriches and masks
/// the event, then dispatches it to the configured sink under the configured failure policy.
/// </summary>
public interface IAuditRecorder
{
    /// <summary>Enriches, masks, and dispatches an audit event.</summary>
    /// <param name="auditEvent">The event to record.</param>
    /// <param name="cancellationToken">A token that may cancel the operation.</param>
    /// <returns>A task completing when the event has been dispatched.</returns>
    Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default);
}
