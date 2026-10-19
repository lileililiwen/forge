namespace Platform.Domain;

/// <summary>
/// Opt-in marker for entities that support soft deletion. The platform
/// exposes the marker data only; it applies no query filters, mappings,
/// or delete behavior. Persistence remains application-owned.
/// </summary>
public interface ISoftDeletable
{
    /// <summary>
    /// Gets a value indicating whether the entity is deleted.
    /// </summary>
    bool IsDeleted { get; }

    /// <summary>
    /// Gets the UTC timestamp when the entity was deleted.
    /// </summary>
    DateTimeOffset? DeletedOnUtc { get; }

    /// <summary>
    /// Gets the identifier of the actor that deleted the entity.
    /// </summary>
    string? DeletedBy { get; }
}
