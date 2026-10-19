namespace Platform.Domain;

/// <summary>
/// Optional base implementation of <see cref="IEntity{TId}"/>.
/// Applications may implement <see cref="IEntity{TId}"/> directly instead
/// of inheriting; the interface remains the preferred integration seam.
/// </summary>
/// <typeparam name="TId">The type of the entity identifier.</typeparam>
public abstract class Entity<TId> : IEntity<TId>
{
    /// <summary>
    /// Initializes a new instance of the <see cref="Entity{TId}"/> class.
    /// </summary>
    /// <param name="id">The entity identifier.</param>
    protected Entity(TId id)
    {
        Id = id;
    }

    /// <summary>
    /// Gets the entity identifier.
    /// </summary>
    public TId Id { get; protected set; }
}
