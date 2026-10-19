namespace Platform.Domain;

/// <summary>
/// Represents the consistency boundary of a domain model: an entity that
/// also carries transient domain events.
/// </summary>
/// <typeparam name="TId">The type of the aggregate identifier.</typeparam>
public interface IAggregateRoot<out TId> : IEntity<TId>, IHasDomainEvents
{
}
