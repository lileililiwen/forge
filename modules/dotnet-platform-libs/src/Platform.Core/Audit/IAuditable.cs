namespace Platform.Core.Audit;

/// <summary>
/// Captures the audit metadata shared across application entities. The
/// interface is intentionally minimal so product types can implement it
/// without inheriting from a platform base class.
/// </summary>
public interface IAuditable
{
    /// <summary>
    /// Gets the UTC time at which the entity was created.
    /// </summary>
    DateTimeOffset CreatedAt { get; }

    /// <summary>
    /// Gets the subject identifier of the caller that created the
    /// entity, or <c>null</c> when unknown.
    /// </summary>
    string? CreatedBy { get; }

    /// <summary>
    /// Gets the UTC time at which the entity was last updated, or
    /// <c>null</c> when the entity has not been modified since
    /// creation.
    /// </summary>
    DateTimeOffset? UpdatedAt { get; }

    /// <summary>
    /// Gets the subject identifier of the caller that last updated the
    /// entity, or <c>null</c> when the entity has not been modified
    /// since creation or the updater is unknown.
    /// </summary>
    string? UpdatedBy { get; }
}
