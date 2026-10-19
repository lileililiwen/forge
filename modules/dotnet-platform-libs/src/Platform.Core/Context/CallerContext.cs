namespace Platform.Core.Context;

/// <summary>
/// Represents the calling party for an operation. Both the subject and
/// the tenant are optional and remain unverified — the contract only
/// describes what was observed, not what is authorized.
/// </summary>
/// <param name="SubjectId">The optional stable identifier of the caller.</param>
/// <param name="TenantId">The optional stable identifier of the tenant the caller operates in.</param>
public sealed record CallerContext(string? SubjectId = null, string? TenantId = null)
{
    /// <summary>
    /// Gets a value indicating whether the caller has no observed
    /// subject identifier.
    /// </summary>
    public bool IsAnonymous => string.IsNullOrWhiteSpace(SubjectId);

    /// <summary>
    /// Gets a value indicating whether the caller is scoped to a
    /// specific tenant.
    /// </summary>
    public bool HasTenant => !string.IsNullOrWhiteSpace(TenantId);

    /// <summary>
    /// Gets a context with no subject and no tenant. Use this when no
    /// caller information is available.
    /// </summary>
    public static CallerContext Anonymous { get; } = new();
}
