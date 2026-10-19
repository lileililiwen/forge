namespace Platform.Auditing.Contracts;

/// <summary>The default retention policy that retains every event. Applications replace it with a bounded schedule.</summary>
public sealed class RetainAllAuditRetentionPolicy : IAuditRetentionPolicy
{
    /// <inheritdoc />
    public bool ShouldRetain(AuditEvent auditEvent) => true;
}
