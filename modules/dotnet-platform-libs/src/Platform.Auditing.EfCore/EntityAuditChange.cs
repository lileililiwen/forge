namespace Platform.Auditing.EfCore;

/// <summary>The kind of change observed for a single property.</summary>
public enum EntityAuditChangeKind
{
    /// <summary>The entity (and the property) was created.</summary>
    Added = 0,

    /// <summary>An existing property value was changed.</summary>
    Modified = 1,

    /// <summary>The entity (and the property) was deleted.</summary>
    Deleted = 2,
}

/// <summary>A single masked property change captured for an audited entity.</summary>
/// <param name="Property">The property name.</param>
/// <param name="Kind">The change kind.</param>
/// <param name="OldValue">The masked prior value, or <c>null</c> when not applicable.</param>
/// <param name="NewValue">The masked new value, or <c>null</c> when not applicable.</param>
public sealed record EntityAuditChange(string Property, EntityAuditChangeKind Kind, string? OldValue, string? NewValue);
