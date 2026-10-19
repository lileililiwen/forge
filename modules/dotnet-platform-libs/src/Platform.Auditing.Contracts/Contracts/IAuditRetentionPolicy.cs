namespace Platform.Auditing.Contracts;

/// <summary>
/// Decides whether a recorded event should be retained. The platform does not own an audit store,
/// schedule, or retention job; applications supply the policy.
/// </summary>
public interface IAuditRetentionPolicy
{
    /// <summary>Returns <c>true</c> when the event should be retained.</summary>
    /// <param name="auditEvent">The event being considered for retention.</param>
    /// <returns><c>true</c> to retain; otherwise <c>false</c>.</returns>
    bool ShouldRetain(AuditEvent auditEvent);
}
