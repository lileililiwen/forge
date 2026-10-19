using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.SignalR;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Context;
using Platform.Realtime.Authorization;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// SignalR hub filter that runs the application connection-authorization decision
/// on every connect. A denied decision throws, which closes the connection before
/// any hub method can execute. Register with
/// <c>AddPlatformRealtimeSignalR</c>; applications must also register an
/// <see cref="IRealtimeConnectionAuthorizer"/> (a fail-closed default is used
/// otherwise).
/// </summary>
public sealed class RealtimeHubAuthorizationFilter : IHubFilter
{
    /// <inheritdoc />
    public static async Task OnConnectedAsync(HubLifetimeContext context, Func<Task> next)
    {
        var authorizer = context.ServiceProvider.GetRequiredService<IRealtimeConnectionAuthorizer>();
        var resolver = context.ServiceProvider.GetRequiredService<Platform.Realtime.AspNetCore.Caller.IRealtimeCallerResolver>();

        var http = context.Context.GetHttpContext();
        var caller = http is not null
            ? await resolver.ResolveAsync(http, default).ConfigureAwait(false)
            : CallerContext.Anonymous;

        await RealtimeHubAuthorization.AuthorizeOrThrowAsync(
            new RealtimeConnectionRequest { ConnectionId = context.Context.ConnectionId, Caller = caller },
            authorizer).ConfigureAwait(false);

        await next().ConfigureAwait(false);
    }

    /// <inheritdoc />
    public static Task InvokeMethodAsync(HubInvocationContext invocationContext, Func<Task> next) => next();

    /// <inheritdoc />
    public static Task OnDisconnectedAsync(HubLifetimeContext context, Exception? exception, Func<Task> next) => next();
}
