using Microsoft.AspNetCore.Http;
using Platform.Core.Context;
using Platform.Realtime;
using Platform.Realtime.Messages;
using Platform.Realtime.Tenant;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// SSE implementation of <see cref="IRealtimeMessageSink"/>. Each write is
/// checked against the application tenant-routing decision and the configured
/// payload bound; rejected messages are silently dropped so no cross-tenant or
/// oversized frame reaches the client.
/// </summary>
public sealed class SseMessageSink : IRealtimeMessageSink
{
    private readonly HttpResponse _response;
    private readonly RealtimeConnectionOptions _options;
    private readonly CallerContext _caller;
    private readonly IRealtimeTenantRouter _router;

    /// <summary>Initializes a new per-connection sink.</summary>
    public SseMessageSink(
        HttpResponse response,
        RealtimeConnectionOptions options,
        CallerContext caller,
        IRealtimeTenantRouter router)
    {
        ArgumentNullException.ThrowIfNull(response);
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(router);
        _response = response;
        _options = options;
        _caller = caller;
        _router = router;
    }

    /// <inheritdoc />
    public async ValueTask WriteAsync(RealtimeMessage message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(message);

        var route = new RealtimeTenantRoute
        {
            TargetTenantId = message.TargetTenantId,
            Caller = _caller,
        };

        if (!await _router.IsRouteAllowedAsync(route, cancellationToken).ConfigureAwait(false))
            return;

        if (message.GetPayloadByteCount() > _options.MaxPayloadBytes)
            return;

        var body = message.PayloadText ?? string.Empty;
        var line = message.Channel is { Length: > 0 }
            ? $"event: {message.Channel}\ndata: {body}\n\n"
            : $"data: {body}\n\n";

        await _response.WriteAsync(line, cancellationToken).ConfigureAwait(false);
        await _response.Body.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    /// <summary>
    /// Writes an SSE comment heartbeat. The comment carries no data but keeps the
    /// connection open through proxies and idle timeouts.
    /// </summary>
    public async ValueTask WriteHeartbeatAsync(CancellationToken cancellationToken = default)
    {
        await _response.WriteAsync(":\n\n", cancellationToken).ConfigureAwait(false);
        await _response.Body.FlushAsync(cancellationToken).ConfigureAwait(false);
    }
}
