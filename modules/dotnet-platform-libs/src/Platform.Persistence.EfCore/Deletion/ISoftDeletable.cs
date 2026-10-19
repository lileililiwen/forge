namespace Platform.Persistence.EfCore.Deletion;

/// <summary>Marks an entity whose deletion is represented by metadata.</summary>
public interface ISoftDeletable
{
    /// <summary>Gets or sets whether the entity is deleted.</summary>
    bool IsDeleted { get; set; }

    /// <summary>Gets or sets the deletion time.</summary>
    DateTimeOffset? DeletedAt { get; set; }

    /// <summary>Gets or sets the subject that deleted the entity.</summary>
    string? DeletedBy { get; set; }
}
