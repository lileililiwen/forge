using System.Collections.Concurrent;
using Platform.Identity.Contracts;

namespace Platform.Identity.Testing;

/// <summary>Records every identity audit event in memory for test assertions.</summary>
public sealed class RecordingIdentityAuditHook : IIdentityAuditHook
{
    /// <summary>The recorded events.</summary>
    public ConcurrentBag<IdentityAuditEvent> Events { get; } = new();

    /// <inheritdoc />
    public ValueTask RecordAsync(IdentityAuditEvent auditEvent, CancellationToken cancellationToken = default)
    {
        Events.Add(auditEvent);
        return ValueTask.CompletedTask;
    }
}
