namespace Platform.Auditing.Contracts;

/// <summary>The default enricher that returns the event unchanged.</summary>
public sealed class NoOpAuditEnricher : IAuditEnricher
{
    /// <inheritdoc />
    public AuditEvent Enrich(AuditEvent auditEvent) => auditEvent;
}
