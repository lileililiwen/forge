using Microsoft.AspNetCore.Http;
using Platform.Core.Context;

namespace Platform.Realtime.AspNetCore.Caller;

/// <summary>
/// Resolves the <see cref="CallerContext"/> for a realtime connection from the
/// incoming <see cref="HttpContext"/>. The platform does not know the host's
/// authentication scheme, so applications supply this to map claims/headers to a
/// tenant and subject.
/// </summary>
public interface IRealtimeCallerResolver
{
    /// <summary>
    /// Returns the caller context for the current connection request.
    /// </summary>
    /// <param name="http">The inbound HTTP context.</param>
    /// <param name="cancellationToken">A token that cancels the resolution.</param>
    /// <returns>The resolved caller context.</returns>
    ValueTask<CallerContext> ResolveAsync(HttpContext http, CancellationToken cancellationToken = default);
}
