namespace Platform.Domain;

/// <summary>
/// Represents a transient domain event raised by an entity or aggregate.
/// Events are plain application state; the platform provides no dispatch,
/// persistence, or outbox guarantees.
/// </summary>
public interface IDomainEvent
{
    /// <summary>
    /// Gets the unique event identifier.
    /// </summary>
    Guid EventId { get; }

    /// <summary>
    /// Gets the UTC timestamp when the event occurred.
    /// </summary>
    DateTimeOffset OccurredOnUtc { get; }

    /// <summary>
    /// Gets the optional correlation identifier for tracing across boundaries.
    /// </summary>
    string? CorrelationId { get; }

    /// <summary>
    /// Gets the optional tenant identifier associated with the event.
    /// </summary>
    string? TenantId { get; }
}
