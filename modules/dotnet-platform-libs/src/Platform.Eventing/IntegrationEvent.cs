namespace Platform.Eventing;

/// <summary>
/// Base record for typed integration events. Concrete events MUST
/// inherit from this record so the bus can attach a stable
/// <see cref="EventId"/>, an <see cref="OccurredAt"/> timestamp read
/// from the platform clock, and an optional <see cref="CorrelationId"/>.
/// </summary>
/// <param name="EventId">The stable, publisher-assigned event identifier.</param>
/// <param name="OccurredAt">The UTC time the event occurred.</param>
/// <param name="CorrelationId">The optional correlation identifier.</param>
public abstract record IntegrationEvent(
    Guid EventId,
    DateTimeOffset OccurredAt,
    string? CorrelationId) : IIntegrationEvent
{
    /// <summary>
    /// Initializes a derived event with a fresh <see cref="EventId"/>
    /// and the supplied clock-anchored <see cref="OccurredAt"/> time.
    /// The <see cref="CorrelationId"/> is <c>null</c>.
    /// </summary>
    /// <param name="occurredAt">The UTC time the event occurred.</param>
    protected IntegrationEvent(DateTimeOffset occurredAt)
        : this(EventId: Guid.NewGuid(), OccurredAt: occurredAt, CorrelationId: null)
    {
    }

    /// <summary>
    /// Initializes a derived event with a fresh <see cref="EventId"/>,
    /// the supplied clock-anchored <see cref="OccurredAt"/> time, and
    /// the supplied <see cref="CorrelationId"/>.
    /// </summary>
    /// <param name="occurredAt">The UTC time the event occurred.</param>
    /// <param name="correlationId">The correlation identifier.</param>
    protected IntegrationEvent(DateTimeOffset occurredAt, string? correlationId)
        : this(EventId: Guid.NewGuid(), OccurredAt: occurredAt, CorrelationId: correlationId)
    {
    }
}
