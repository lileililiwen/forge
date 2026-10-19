using System.Collections.Generic;

namespace Platform.Auditing.Contracts;

/// <summary>
/// An immutable, normalized audit record. It carries the action, outcome, severity, occurrence
/// time, correlation, optional tenant/subject, and bounded, already-masked metadata. A persistence
/// provider is never required to construct or emit an event.
/// </summary>
public sealed record AuditEvent
{
    /// <summary>The stable action name, for example <c>http.request</c> or <c>entity.updated</c>.</summary>
    public string Action { get; init; }

    /// <summary>The broad category of the event, for example <c>http</c>, <c>security</c>, or <c>entity</c>.</summary>
    public string Category { get; init; }

    /// <summary>The normalized outcome of the action.</summary>
    public AuditOutcome Outcome { get; init; }

    /// <summary>The severity of the event.</summary>
    public AuditSeverity Severity { get; init; }

    /// <summary>The UTC time at which the event occurred.</summary>
    public DateTimeOffset OccurredAt { get; init; }

    /// <summary>Optional correlation identifier linking the event to a request or flow.</summary>
    public string? CorrelationId { get; init; }

    /// <summary>Optional tenant identifier, when known.</summary>
    public string? TenantId { get; init; }

    /// <summary>Optional subject (actor) identifier, when known.</summary>
    public string? SubjectId { get; init; }

    /// <summary>Optional logical source of the event, for example the host or assembly name.</summary>
    public string? Source { get; init; }

    /// <summary>Bounded, safe metadata. Values must already be masked before assignment.</summary>
    public IReadOnlyDictionary<string, string> Metadata { get; init; }

    private AuditEvent(string action, string category)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(action);
        ArgumentException.ThrowIfNullOrWhiteSpace(category);
        Action = action;
        Category = category;
        Outcome = AuditOutcome.Unknown;
        Severity = AuditSeverity.Information;
        OccurredAt = DateTimeOffset.MinValue;
        Metadata = new Dictionary<string, string>();
    }

    /// <summary>
    /// Initializes an empty event for deserialization and validation. Values must be set via the
    /// <c>init</c> properties and should be validated with <see cref="Validate"/> before use.
    /// </summary>
    public AuditEvent()
    {
        Action = string.Empty;
        Category = string.Empty;
        Metadata = new Dictionary<string, string>();
    }

    /// <summary>Creates an audit event with the minimum required fields.</summary>
    /// <param name="action">The action name. Cannot be empty.</param>
    /// <param name="category">The category. Cannot be empty.</param>
    /// <param name="outcome">The outcome.</param>
    /// <param name="occurredAt">The UTC occurrence time.</param>
    /// <param name="severity">The severity.</param>
    public static AuditEvent Create(
        string action,
        string category,
        AuditOutcome outcome = AuditOutcome.Unknown,
        DateTimeOffset occurredAt = default,
        AuditSeverity severity = AuditSeverity.Information)
    {
        return new AuditEvent(action, category)
        {
            Outcome = outcome,
            Severity = severity,
            OccurredAt = occurredAt == default ? DateTimeOffset.UtcNow : occurredAt,
        };
    }

    /// <summary>Returns a copy with an additional metadata entry, replacing any existing value.</summary>
    public AuditEvent WithMetadata(string key, string? value)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);
        var next = new Dictionary<string, string>(Metadata) { [key] = value ?? string.Empty };
        return this with { Metadata = next };
    }

    /// <summary>Returns a copy with the supplied metadata merged over the existing entries.</summary>
    /// <param name="metadata">The additional metadata. Cannot be <c>null</c>.</param>
    /// <returns>A copy with the merged metadata.</returns>
    public AuditEvent WithMetadata(IReadOnlyDictionary<string, string> metadata)
    {
        ArgumentNullException.ThrowIfNull(metadata);
        var next = new Dictionary<string, string>(Metadata);
        foreach (var pair in metadata)
        {
            next[pair.Key] = pair.Value;
        }

        return this with { Metadata = next };
    }

    /// <summary>Returns a copy with the supplied correlation identifier.</summary>
    public AuditEvent WithCorrelation(string? correlationId) => this with { CorrelationId = correlationId };

    /// <summary>Returns a copy with the supplied subject and tenant identifiers.</summary>
    public AuditEvent WithActor(string? subjectId, string? tenantId = null)
        => this with { SubjectId = subjectId, TenantId = tenantId };

    /// <summary>Returns a copy with the supplied logical source.</summary>
    public AuditEvent WithSource(string? source) => this with { Source = source };

    /// <summary>Returns a copy with the supplied severity.</summary>
    public AuditEvent WithSeverity(AuditSeverity severity) => this with { Severity = severity };

    /// <summary>Returns a copy with the supplied outcome.</summary>
    public AuditEvent WithOutcome(AuditOutcome outcome) => this with { Outcome = outcome };

    /// <summary>Returns a copy with the supplied category.</summary>
    public AuditEvent WithCategory(string category)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(category);
        return this with { Category = category };
    }

    /// <summary>Validates the event. Returns human-readable errors; empty when valid.</summary>
    public IReadOnlyList<string> Validate()
    {
        var errors = new List<string>();
        if (string.IsNullOrWhiteSpace(Action)) errors.Add("Audit action must not be empty.");
        if (string.IsNullOrWhiteSpace(Category)) errors.Add("Audit category must not be empty.");
        if (OccurredAt == DateTimeOffset.MinValue) errors.Add("Audit occurrence time must be set.");
        foreach (var key in Metadata.Keys)
        {
            if (string.IsNullOrWhiteSpace(key)) errors.Add("Audit metadata keys must not be empty.");
        }

        return errors;
    }
}
