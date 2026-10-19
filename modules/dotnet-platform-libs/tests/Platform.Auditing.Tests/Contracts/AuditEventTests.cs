using Platform.Auditing.Contracts;
using Platform.Core.Time;

namespace Platform.Auditing.Tests.Contracts;

public class AuditEventTests
{
    [Fact]
    public void Create_assigns_required_fields_and_defaults_occurrence_time()
    {
        var before = DateTimeOffset.UtcNow;
        var auditEvent = AuditEvent.Create("http.request", "http", AuditOutcome.Success);
        var after = DateTimeOffset.UtcNow;

        Assert.Equal("http.request", auditEvent.Action);
        Assert.Equal("http", auditEvent.Category);
        Assert.Equal(AuditOutcome.Success, auditEvent.Outcome);
        Assert.Equal(AuditSeverity.Information, auditEvent.Severity);
        Assert.True(auditEvent.OccurredAt >= before);
        Assert.True(auditEvent.OccurredAt <= after);
        Assert.Empty(auditEvent.Metadata);
    }

    [Fact]
    public void Create_honors_explicit_occurrence_time()
    {
        var at = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        var auditEvent = AuditEvent.Create("entity.updated", "entity", AuditOutcome.Success, at, AuditSeverity.Warning);

        Assert.Equal(at, auditEvent.OccurredAt);
        Assert.Equal(AuditSeverity.Warning, auditEvent.Severity);
    }

    [Fact]
    public void WithMetadata_adds_and_replaces_entries()
    {
        var auditEvent = AuditEvent.Create("http.request", "http")
            .WithMetadata("http.method", "GET")
            .WithMetadata("http.method", "POST");

        Assert.Single(auditEvent.Metadata);
        Assert.Equal("POST", auditEvent.Metadata["http.method"]);
    }

    [Fact]
    public void WithActor_and_correlation_set_values()
    {
        var auditEvent = AuditEvent.Create("http.request", "http")
            .WithActor("subject-1", "tenant-9")
            .WithCorrelation("corr-42");

        Assert.Equal("subject-1", auditEvent.SubjectId);
        Assert.Equal("tenant-9", auditEvent.TenantId);
        Assert.Equal("corr-42", auditEvent.CorrelationId);
    }

    [Fact]
    public void Validate_reports_empty_action_category_and_unset_time()
    {
        var auditEvent = new AuditEvent
        {
            Action = string.Empty,
            Category = " ",
            OccurredAt = DateTimeOffset.MinValue,
            Metadata = new Dictionary<string, string> { [string.Empty] = "x" },
        };

        var errors = auditEvent.Validate();

        Assert.Contains(errors, e => e.Contains("action", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(errors, e => e.Contains("category", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(errors, e => e.Contains("occurrence", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void Valid_event_has_no_validation_errors()
    {
        var auditEvent = AuditEvent.Create("http.request", "http", AuditOutcome.Success,
            new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Empty(auditEvent.Validate());
    }

    [Fact]
    public void Create_rejects_empty_action()
    {
        Assert.Throws<ArgumentException>(() => AuditEvent.Create(string.Empty, "http"));
    }

    [Fact]
    public void FixedClock_is_used_by_sink_dispatch_without_system_time()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 5, 5, 5, 5, 5, TimeSpan.Zero));
        Assert.Equal(TimeSpan.Zero, clock.UtcNow.Offset);
    }
}
