using Platform.Core.Audit;

namespace Platform.Persistence.EfCore.Audit;

/// <summary>Writable audit metadata used by the optional save interceptor.</summary>
public interface IAuditableEntity : IAuditable
{
    /// <summary>Sets the creation time.</summary>
    new DateTimeOffset CreatedAt { get; set; }

    /// <summary>Sets the creation subject.</summary>
    new string? CreatedBy { get; set; }

    /// <summary>Sets the update time.</summary>
    new DateTimeOffset? UpdatedAt { get; set; }

    /// <summary>Sets the update subject.</summary>
    new string? UpdatedBy { get; set; }
}

/// <summary>Provides the subject used by persistence conventions.</summary>
public interface IActorAccessor
{
    /// <summary>Gets the current subject identifier, when known.</summary>
    string? SubjectId { get; }
}
