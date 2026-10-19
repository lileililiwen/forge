using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Realtime.Status;

namespace Platform.Realtime.AspNetCore.Status;

/// <summary>
/// Endpoint mapping for safe realtime provider status.
/// </summary>
public static class RealtimeProviderStatusEndpointExtensions
{
    /// <summary>
    /// Maps a read-only status endpoint that exposes safe transport health
    /// (availability and backplane usage) without leaking topology or credentials.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <param name="pattern">The route pattern. Defaults to <c>/realtime/status</c>.</param>
    /// <returns>The same builder for chaining.</returns>
    public static IEndpointRouteBuilder MapPlatformRealtimeStatus(
        this IEndpointRouteBuilder endpoints,
        string pattern = "/realtime/status")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(pattern))
            throw new ArgumentException("A non-empty status route pattern is required.", nameof(pattern));

        endpoints.MapGet(pattern, async (HttpContext http, CancellationToken ct) =>
        {
            var status = http.RequestServices.GetRequiredService<IRealtimeProviderStatus>();
            var snapshot = await status.GetStatusAsync(ct).ConfigureAwait(false);
            await http.Response.WriteAsJsonAsync(snapshot, ct).ConfigureAwait(false);
        });

        return endpoints;
    }
}
