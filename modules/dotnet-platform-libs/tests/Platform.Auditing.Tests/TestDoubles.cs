using Platform.Auditing.Contracts;

namespace Platform.Auditing.Tests;

/// <summary>Shared test doubles for the auditing test suite.</summary>
internal static class AuditTestDoubles
{
    public static DateTimeOffset FixedUtc() => new(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
}

/// <summary>Captures every event it receives.</summary>
internal sealed class CapturingAuditSink : IAuditSink
{
    private readonly List<AuditEvent> _events = new();
    public IReadOnlyList<AuditEvent> Recorded => _events;
    public Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default)
    {
        _events.Add(auditEvent);
        return Task.CompletedTask;
    }
}

/// <summary>Throws on every record so failure policies can be exercised.</summary>
internal sealed class ThrowingAuditSink : IAuditSink
{
    public IReadOnlyList<AuditEvent> Recorded { get; } = new List<AuditEvent>();
    public Task RecordAsync(AuditEvent auditEvent, CancellationToken cancellationToken = default)
        => throw new InvalidOperationException("audit sink unavailable");
}

/// <summary>Adds a fixed metadata entry to every event.</summary>
internal sealed class StaticEnricher : IAuditEnricher
{
    private readonly string _key;
    private readonly string _value;
    public StaticEnricher(string key, string value)
    {
        _key = key;
        _value = value;
    }

    public AuditEvent Enrich(AuditEvent auditEvent) => auditEvent.WithMetadata(_key, _value);
}
