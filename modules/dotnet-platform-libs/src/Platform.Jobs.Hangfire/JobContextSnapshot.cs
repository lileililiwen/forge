namespace Platform.Jobs.Hangfire;

/// <summary>
/// The tenant and subject identity captured when a job is created and
/// restored inside the job's execution scope. Only opaque identifiers are
/// carried; the platform never captures claims, tokens, or tenant records.
/// </summary>
/// <param name="TenantId">The ambient tenant identifier, or <c>null</c> when the job is tenant-neutral.</param>
/// <param name="SubjectId">The ambient subject identifier, or <c>null</c> when no subject is active.</param>
public sealed record JobContextSnapshot(string? TenantId, string? SubjectId);
