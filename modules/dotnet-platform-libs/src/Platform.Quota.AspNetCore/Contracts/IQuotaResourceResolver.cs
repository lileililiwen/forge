using Microsoft.AspNetCore.Http;
using Platform.Quota.Contracts;

namespace Platform.Quota.AspNetCore.Contracts;

/// <summary>A single quota operation an application applies to a request.</summary>
/// <param name="Resource">Opaque application-defined resource.</param>
/// <param name="Limit">Application-owned capacity for the window.</param>
/// <param name="Amount">Units consumed by this request.</param>
/// <param name="Window">Explicit UTC accounting window.</param>
/// <param name="Reserve">When true, the middleware reserves and settles/releases across the pipeline; otherwise it checks only.</param>
public sealed record QuotaRequest(QuotaResource Resource, long Limit, long Amount, QuotaWindow Window, bool Reserve = false);

/// <summary>
/// Selects the quota resources a request consumes. The application owns which resources apply, their
/// limits, units, and accounting windows. Returning an empty list exempts the request from quota.
/// </summary>
public interface IQuotaResourceResolver
{
    /// <summary>Resolves the quota operations for the current request and subject.</summary>
    Task<IReadOnlyList<QuotaRequest>> ResolveAsync(HttpContext context, QuotaSubject subject, CancellationToken cancellationToken = default);
}
