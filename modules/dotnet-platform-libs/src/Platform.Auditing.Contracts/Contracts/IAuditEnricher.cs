namespace Platform.Auditing.Contracts;

/// <summary>
/// Adds or adjusts audit metadata immediately before an event is dispatched. Enrichers must not
/// throw and must return a new event (events are immutable). The platform applies enrichers in
/// registration order.
/// </summary>
public interface IAuditEnricher
{
    /// <summary>Returns an enriched copy of <paramref name="auditEvent"/>.</summary>
    /// <param name="auditEvent">The event to enrich.</param>
    /// <returns>The enriched event.</returns>
    AuditEvent Enrich(AuditEvent auditEvent);
}
