using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Platform.Realtime.Delivery;

namespace Platform.Realtime.AspNetCore.Delivery;

/// <summary>
/// Endpoint mapping for the non-durable delivery resynchronization hook.
/// </summary>
public static class RealtimeResyncExtensions
{
    /// <summary>
    /// Maps a POST endpoint that accepts a client
    /// <see cref="RealtimeResyncRequest"/> and forwards it to the application
    /// <see cref="IRealtimeResyncHandler"/>. The platform does not redeliver missed
    /// messages itself; the application owns replay. When no handler is registered
    /// the endpoint returns <c>404</c> so clients learn resync is not configured.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <param name="pattern">The route pattern. Defaults to <c>/realtime/resync</c>.</param>
    /// <returns>The same builder for chaining.</returns>
    public static IEndpointRouteBuilder MapPlatformRealtimeResync(
        this IEndpointRouteBuilder endpoints,
        string pattern = "/realtime/resync")
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(pattern))
            throw new ArgumentException("A non-empty resync route pattern is required.", nameof(pattern));

        endpoints.MapPost(pattern, async (HttpContext http, CancellationToken ct) =>
        {
            var handler = http.RequestServices.GetService<IRealtimeResyncHandler>();
            if (handler is null)
            {
                http.Response.StatusCode = StatusCodes.Status404NotFound;
                await http.Response.WriteAsJsonAsync(
                    new { error = "realtime_resync_not_configured" }, ct).ConfigureAwait(false);
                return;
            }

            var request = await http.Request.ReadFromJsonAsync<RealtimeResyncRequest>(ct).ConfigureAwait(false);
            if (request is null)
            {
                http.Response.StatusCode = StatusCodes.Status400BadRequest;
                return;
            }

            await handler.HandleAsync(request, ct).ConfigureAwait(false);
            http.Response.StatusCode = StatusCodes.Status202Accepted;
        });

        return endpoints;
    }
}
