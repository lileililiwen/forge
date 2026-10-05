namespace Platform.Privacy.Contracts;

/// <summary>A subject's consent decision under one policy version.</summary>
/// <param name="SubjectId">Application-defined subject identifier.</param>
/// <param name="Granted">Optional categories the subject granted.</param>
/// <param name="GrantedAt">When the decision was recorded.</param>
/// <param name="PolicyVersion">Policy version the decision was granted under.</param>
public sealed record ConsentRecord(
    string SubjectId,
    IReadOnlySet<ConsentCategory> Granted,
    DateTimeOffset GrantedAt,
    int PolicyVersion);
