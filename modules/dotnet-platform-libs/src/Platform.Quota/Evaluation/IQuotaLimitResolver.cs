using Platform.Quota.Contracts;

namespace Platform.Quota.Evaluation;

/// <summary>Optional application bridge from an entitlement decision to a quota limit.</summary>
public interface IQuotaLimitResolver
{
    /// <summary>Resolves an application-owned limit for a subject and resource.</summary>
    Task<QuotaLimitResolution> ResolveAsync(QuotaSubject subject, QuotaResource resource, CancellationToken cancellationToken = default);
}

/// <summary>Resolved quota limit with opaque unit semantics.</summary>
public sealed record QuotaLimitResolution(bool Allowed, long Limit, string? Unit = null)
{
    /// <summary>Creates a denied resolution.</summary>
    public static QuotaLimitResolution Denied(string? unit = null) => new(false, 0, unit);
}
