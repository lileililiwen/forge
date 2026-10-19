namespace Platform.Privacy.Contracts;

/// <summary>Persistence for consent records. Implementations must be thread-safe.</summary>
public interface IConsentStore
{
    /// <summary>Returns the record for a subject, or null when none exists.</summary>
    Task<ConsentRecord?> GetAsync(string subjectId, CancellationToken cancellationToken = default);

    /// <summary>Replaces the record for a subject.</summary>
    Task SetAsync(ConsentRecord record, CancellationToken cancellationToken = default);
}
