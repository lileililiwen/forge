namespace Platform.Core.Tenancy;

/// <summary>
/// Resolves the tenant for the current operation. Implementations are
/// application-owned; the platform never inspects request headers,
/// claims, or configuration directly.
/// </summary>
public interface ITenantResolver
{
    /// <summary>
    /// Resolves the tenant for the supplied <paramref name="context"/>.
    /// The <paramref name="context"/> is opaque to the platform; the
    /// application decides what it carries (HTTP context, background
    /// context, message envelope, and so on).
    /// </summary>
    /// <param name="context">Opaque ambient context supplied by the caller.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    ValueTask<TenantResolutionResult> ResolveAsync(object? context, CancellationToken cancellationToken = default);
}
