namespace Platform.Domain;

/// <summary>
/// Exposes the transient domain events raised by an entity or aggregate.
/// Recorded events are inspectable application state only; the platform
/// never dispatches them. Transaction and delivery policy remain
/// application-owned.
/// </summary>
public interface IHasDomainEvents
{
    /// <summary>
    /// Gets the recorded domain events in insertion order.
    /// </summary>
    IReadOnlyCollection<IDomainEvent> DomainEvents { get; }

    /// <summary>
    /// Records a domain event for later application-owned dispatch.
    /// </summary>
    /// <param name="domainEvent">The domain event to record.</param>
    void AddDomainEvent(IDomainEvent domainEvent);

    /// <summary>
    /// Clears all recorded domain events without dispatching them.
    /// </summary>
    void ClearDomainEvents();
}
