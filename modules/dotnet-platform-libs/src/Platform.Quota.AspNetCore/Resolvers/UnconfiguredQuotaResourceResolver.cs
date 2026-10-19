using Microsoft.AspNetCore.Http;
using Platform.Quota.AspNetCore.Contracts;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Resolvers;

/// <summary>
/// Fail-closed default used until the application registers an <see cref="IQuotaResourceResolver"/>.
/// It refuses every request so an unconfigured host never silently bypasses quota. Replace it with
/// the application-owned resolver after calling <c>AddPlatformQuotaAspNetCore</c>.
/// </summary>
public sealed class UnconfiguredQuotaResourceResolver : IQuotaResourceResolver
{
    /// <inheritdoc />
    public Task<IReadOnlyList<QuotaRequest>> ResolveAsync(HttpContext context, QuotaSubject subject, CancellationToken cancellationToken = default)
        => throw new InvalidOperationException(
            "No IQuotaResourceResolver is registered. Register an application-owned IQuotaResourceResolver after AddPlatformQuotaAspNetCore so the host enforces quota explicitly.");
}
