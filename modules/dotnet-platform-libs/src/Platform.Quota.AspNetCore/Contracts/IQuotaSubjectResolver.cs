using Microsoft.AspNetCore.Http;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Contracts;

/// <summary>Result of resolving the opaque quota subject for a request.</summary>
/// <param name="Subject">The resolved subject, or null when context is missing.</param>
public readonly record struct QuotaSubjectResolution(QuotaSubject? Subject)
{
    /// <summary>True when a subject was resolved.</summary>
    public bool Resolved => Subject is not null;
    /// <summary>A missing-context result.</summary>
    public static QuotaSubjectResolution Missing => new(null);
}

/// <summary>
/// Resolves the opaque quota subject (and any tenant context) for a request. Applications own this
/// mapping; the platform never inspects product identity. Return <see cref="QuotaSubjectResolution.Missing"/>
/// when the required context cannot be resolved so the configured <see cref="MissingContextPolicy"/> applies.
/// </summary>
public interface IQuotaSubjectResolver
{
    /// <summary>Resolves the subject for the current request.</summary>
    Task<QuotaSubjectResolution> ResolveAsync(HttpContext context, CancellationToken cancellationToken = default);
}
