namespace Platform.Domain;

/// <summary>
/// Optional base record for <see cref="IDomainEvent"/> implementations with
/// correlation and tenant context. Carries no dispatch behavior.
/// </summary>
/// <param name="EventId">The unique event identifier.</param>
/// <param name="OccurredOnUtc">The UTC timestamp when the event occurred.</param>
/// <param name="CorrelationId">The optional correlation identifier.</param>
/// <param name="TenantId">The optional tenant identifier.</param>
public abstract record DomainEvent(
    Guid EventId,
    DateTimeOffset OccurredOnUtc,
    string? CorrelationId = null,
    string? TenantId = null) : IDomainEvent
{
    /// <summary>
    /// Creates a new domain event using the provided factory, supplying a
    /// fresh identifier and the current UTC timestamp.
    /// </summary>
    /// <typeparam name="T">The domain event type to create.</typeparam>
    /// <param name="factory">Factory that creates the event from the generated id and timestamp.</param>
    /// <returns>The created domain event.</returns>
    public static T Create<T>(Func<Guid, DateTimeOffset, T> factory)
        where T : DomainEvent
    {
        ArgumentNullException.ThrowIfNull(factory);
        return factory(Guid.NewGuid(), DateTimeOffset.UtcNow);
    }
}
