using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.SignalR;
using Microsoft.Extensions.Options;
using Platform.Core.Context;
using Platform.Realtime;
using Platform.Realtime.AspNetCore.Caller;
using Platform.Realtime.Authorization;
using Platform.Realtime.Tenant;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// Optional base class for application SignalR hubs. It performs the platform
/// connection-authorization check on connect (fail-closed) and exposes a
/// tenant-routing-aware <see cref="SendToTenantAsync"/> so hubs never broadcast
/// cross-tenant data without the application routing policy approving it.
/// Applications subclass this hub and map it with <c>MapHub&lt;T&gt;</c> after
/// calling <c>AddPlatformRealtimeSignalR</c>.
/// </summary>
public abstract class RealtimeHubBase : Hub
{
    private readonly IRealtimeConnectionAuthorizer _authorizer;
    private readonly IRealtimeTenantRouter _router;
    private readonly IRealtimeCallerResolver _callerResolver;
    private readonly RealtimeConnectionOptions _options;
    private CallerContext _caller = CallerContext.Anonymous;

    /// <summary>
    /// Initializes a new base hub with the platform realtime services.
    /// </summary>
    protected RealtimeHubBase(
        IRealtimeConnectionAuthorizer authorizer,
        IRealtimeTenantRouter router,
        IRealtimeCallerResolver callerResolver,
        IOptions<RealtimeConnectionOptions> options)
    {
        _authorizer = authorizer ?? throw new ArgumentNullException(nameof(authorizer));
        _router = router ?? throw new ArgumentNullException(nameof(router));
        _callerResolver = callerResolver ?? throw new ArgumentNullException(nameof(callerResolver));
        _options = options?.Value ?? throw new ArgumentNullException(nameof(options));
    }

    /// <inheritdoc />
    public override async Task OnConnectedAsync()
    {
        var http = Context.GetHttpContext();
        _caller = http is not null
            ? await _callerResolver.ResolveAsync(http, default).ConfigureAwait(false)
            : CallerContext.Anonymous;

        await RealtimeHubAuthorization.AuthorizeOrThrowAsync(
            new RealtimeConnectionRequest { ConnectionId = Context.ConnectionId, Caller = _caller },
            _authorizer).ConfigureAwait(false);

        await base.OnConnectedAsync().ConfigureAwait(false);
    }

    /// <summary>
    /// Sends <paramref name="message"/> to the target tenant only if the
    /// application routing policy allows it. Rejected routes are silently dropped.
    /// When <paramref name="targetTenantId"/> is <c>null</c> the message is sent to
    /// all connections (broadcast). Payloads larger than the configured bound are
    /// dropped.
    /// </summary>
    protected async Task SendToTenantAsync(string? targetTenantId, string method, object? message, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(method);

        var allowed = await RealtimeHubRouting.IsRouteAllowedAsync(
            new RealtimeTenantRoute { TargetTenantId = targetTenantId, Caller = _caller },
            _router,
            cancellationToken).ConfigureAwait(false);

        if (!allowed)
            return;

        var payloadText = message?.ToString();
        if (payloadText is not null && payloadText.Length > _options.MaxPayloadBytes)
            return;

        if (targetTenantId is null)
            await Clients.All.SendAsync(method, message, cancellationToken).ConfigureAwait(false);
        else
            await Clients.Group(targetTenantId).SendAsync(method, message, cancellationToken).ConfigureAwait(false);
    }
}
