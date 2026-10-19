namespace Platform.Domain;

/// <summary>
/// Optional base implementation of <see cref="IAggregateRoot{TId}"/> with an
/// insertion-ordered, transient domain-event collection. Recorded events are
/// never dispatched by the platform. Applications may implement
/// <see cref="IAggregateRoot{TId}"/> directly instead of inheriting.
/// </summary>
/// <typeparam name="TId">The type of the aggregate identifier.</typeparam>
public abstract class AggregateRoot<TId> : Entity<TId>, IAggregateRoot<TId>
{
    private readonly List<IDomainEvent> _domainEvents = [];

    /// <summary>
    /// Initializes a new instance of the <see cref="AggregateRoot{TId}"/> class.
    /// </summary>
    /// <param name="id">The aggregate identifier.</param>
    protected AggregateRoot(TId id)
        : base(id)
    {
    }

    /// <summary>
    /// Gets the recorded domain events in insertion order.
    /// </summary>
    public IReadOnlyCollection<IDomainEvent> DomainEvents => _domainEvents.AsReadOnly();

    /// <summary>
    /// Records a domain event for later application-owned dispatch.
    /// </summary>
    /// <param name="domainEvent">The domain event to record.</param>
    public void AddDomainEvent(IDomainEvent domainEvent)
    {
        ArgumentNullException.ThrowIfNull(domainEvent);
        _domainEvents.Add(domainEvent);
    }

    /// <summary>
    /// Clears all recorded domain events without dispatching them.
    /// </summary>
    public void ClearDomainEvents() => _domainEvents.Clear();

    /// <summary>
    /// Records a domain event for later application-owned dispatch.
    /// Intended for use by derived aggregate behaviors.
    /// </summary>
    /// <param name="domainEvent">The domain event to record.</param>
    protected void RaiseDomainEvent(IDomainEvent domainEvent) => AddDomainEvent(domainEvent);
}
