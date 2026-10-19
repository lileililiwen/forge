using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Core.Context;
using Platform.Realtime;
using Platform.Realtime.Tenant;
using Platform.Realtime.AspNetCore.Caller;
using Platform.Realtime.Authorization;
using Platform.Realtime.Context;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// Endpoint mapping for the platform SSE realtime transport.
/// </summary>
public static class RealtimeSseEndpointExtensions
{
    /// <summary>
    /// Maps a Server-Sent Events stream at <paramref name="pattern"/>. The adapter
    /// resolves the caller, runs the application authorization decision (rejecting
    /// with <c>401</c> when denied), enforces the connection limit (rejecting with
    /// <c>503</c> when full), and then streams the application
    /// <see cref="IRealtimeSseSource"/>. Disconnect or request cancellation
    /// releases the connection slot and ends the stream.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <param name="pattern">The route pattern for the SSE stream.</param>
    /// <returns>The same builder for chaining.</returns>
    public static IEndpointRouteBuilder MapPlatformRealtimeSse(
        this IEndpointRouteBuilder endpoints,
        string pattern)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(pattern))
            throw new ArgumentException("A non-empty SSE route pattern is required.", nameof(pattern));

        endpoints.MapGet(pattern, async (HttpContext http, CancellationToken requestCt) =>
        {
            var services = http.RequestServices;
            var options = services.GetRequiredService<IOptions<RealtimeConnectionOptions>>().Value;
            var limiter = services.GetRequiredService<RealtimeConnectionLimiter>();
            var callerResolver = services.GetRequiredService<IRealtimeCallerResolver>();
            var authorizer = services.GetRequiredService<IRealtimeConnectionAuthorizer>();
            var source = services.GetRequiredService<IRealtimeSseSource>();

            var caller = await callerResolver.ResolveAsync(http, requestCt).ConfigureAwait(false);

            var authResult = await authorizer.AuthorizeAsync(
                new RealtimeConnectionRequest { ConnectionId = http.TraceIdentifier, Caller = caller },
                requestCt).ConfigureAwait(false);

            if (!authResult.Allowed)
            {
                http.Response.StatusCode = StatusCodes.Status401Unauthorized;
                await http.Response.WriteAsJsonAsync(
                    new { error = "realtime_unauthorized", detail = authResult.RejectionReason },
                    requestCt).ConfigureAwait(false);
                return;
            }

            if (!limiter.TryAcquire())
            {
                http.Response.StatusCode = StatusCodes.Status503ServiceUnavailable;
                await http.Response.WriteAsJsonAsync(
                    new { error = "realtime_capacity_exceeded" },
                    requestCt).ConfigureAwait(false);
                return;
            }

            try
            {
                http.Response.ContentType = "text/event-stream";
                http.Response.Headers.CacheControl = "no-cache";
                http.Response.Headers.Connection = "keep-alive";
                http.Response.Headers["X-Accel-Buffering"] = "no";
                await http.Response.Body.FlushAsync(requestCt).ConfigureAwait(false);

                var context = new RealtimeConnectionContext
                {
                    ConnectionId = http.TraceIdentifier,
                    Caller = caller,
                    Cancellation = requestCt,
                };
                var router = services.GetRequiredService<IRealtimeTenantRouter>();
                var sink = new SseMessageSink(http.Response, options, caller, router);

                using var linked = CancellationTokenSource.CreateLinkedTokenSource(requestCt);
                var heartbeat = Task.Run(async () =>
                {
                    try
                    {
                        while (!linked.Token.IsCancellationRequested)
                        {
                            await Task.Delay(options.HeartbeatInterval, linked.Token).ConfigureAwait(false);
                            await sink.WriteHeartbeatAsync(linked.Token).ConfigureAwait(false);
                        }
                    }
                    catch (OperationCanceledException)
                    {
                        // Expected when the connection closes.
                    }
                }, linked.Token);

                await source.StreamAsync(context, sink, linked.Token).ConfigureAwait(false);
                linked.Cancel();

                try
                {
                    await heartbeat.ConfigureAwait(false);
                }
                catch (OperationCanceledException)
                {
                    // Expected after the stream ends.
                }
            }
            finally
            {
                limiter.Release();
            }
        });

        return endpoints;
    }
}
