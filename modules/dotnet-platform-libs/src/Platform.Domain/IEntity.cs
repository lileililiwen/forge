namespace Platform.Domain;

/// <summary>
/// Represents an entity with a strongly-typed identifier.
/// Identity equality and persistence remain application-owned;
/// the platform only carries the identifier.
/// </summary>
/// <typeparam name="TId">The type of the entity identifier.</typeparam>
public interface IEntity<out TId>
{
    /// <summary>
    /// Gets the entity identifier.
    /// </summary>
    TId Id { get; }
}
