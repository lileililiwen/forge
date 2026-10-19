using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Platform.Core.Context;
using Platform.Realtime;
using Platform.Realtime.AspNetCore.Caller;
using Platform.Realtime.AspNetCore.Delivery;
using Platform.Realtime.AspNetCore.Sse;
using Platform.Realtime.AspNetCore.SignalR;
using Platform.Realtime.AspNetCore.Status;
using Platform.Realtime.Authorization;
using Platform.Realtime.Context;
using Platform.Realtime.Delivery;
using Platform.Realtime.Messages;
using Platform.Realtime.Tenant;

namespace Platform.Realtime.Tests;

/// <summary>Shared test doubles and a WebApplication builder for the SSE adapter tests.</summary>
internal static class TestDoubles
{
    public sealed class AllowAuthorizer : IRealtimeConnectionAuthorizer
    {
        public Task<RealtimeAuthorizationResult> AuthorizeAsync(RealtimeConnectionRequest request, CancellationToken cancellationToken = default) =>
            Task.FromResult(RealtimeAuthorizationResult.Allow());
    }

    public sealed class DenyAuthorizer : IRealtimeConnectionAuthorizer
    {
        public Task<RealtimeAuthorizationResult> AuthorizeAsync(RealtimeConnectionRequest request, CancellationToken cancellationToken = default) =>
            Task.FromResult(RealtimeAuthorizationResult.Deny("denied by test"));
    }

    public sealed class TenantResolver : IRealtimeCallerResolver
    {
        private readonly string? _tenant;
        public TenantResolver(string? tenant = null) => _tenant = tenant;
        public ValueTask<CallerContext> ResolveAsync(HttpContext http, CancellationToken cancellationToken = default) =>
            new(new CallerContext(SubjectId: "s1", TenantId: _tenant));
    }

    public sealed class SameTenantRouter : IRealtimeTenantRouter
    {
        public ValueTask<bool> IsRouteAllowedAsync(RealtimeTenantRoute route, CancellationToken cancellationToken = default) =>
            new(string.Equals(route.Caller.TenantId, route.TargetTenantId, StringComparison.Ordinal));
    }

    public sealed class ShortSource : IRealtimeSseSource
    {
        public async Task StreamAsync(RealtimeConnectionContext context, IRealtimeMessageSink sink, CancellationToken cancellationToken)
        {
            await sink.WriteAsync(RealtimeMessage.Create(int.MaxValue, channel: "tick", payloadText: "hello"), cancellationToken);
            await Task.Delay(150, cancellationToken);
        }
    }

    public sealed class RoutingSource : IRealtimeSseSource
    {
        public async Task StreamAsync(RealtimeConnectionContext context, IRealtimeMessageSink sink, CancellationToken cancellationToken)
        {
            await sink.WriteAsync(RealtimeMessage.Create(int.MaxValue, channel: "b", payloadText: "broadcast"), cancellationToken);
            await sink.WriteAsync(RealtimeMessage.Create(int.MaxValue, channel: "s", targetTenantId: "t1", payloadText: "same-tenant"), cancellationToken);
            await sink.WriteAsync(RealtimeMessage.Create(int.MaxValue, channel: "x", targetTenantId: "t2", payloadText: "cross-tenant"), cancellationToken);
            await Task.Delay(150, cancellationToken);
        }
    }

    public sealed class PayloadSource : IRealtimeSseSource
    {
        public async Task StreamAsync(RealtimeConnectionContext context, IRealtimeMessageSink sink, CancellationToken cancellationToken)
        {
            await sink.WriteAsync(RealtimeMessage.Create(1000, channel: "big", payloadText: "this is definitely longer than ten bytes indeed"), cancellationToken);
            await sink.WriteAsync(RealtimeMessage.Create(1000, channel: "small", payloadText: "ok"), cancellationToken);
            await Task.Delay(150, cancellationToken);
        }
    }

    public sealed class LongSource : IRealtimeSseSource
    {
        public async Task StreamAsync(RealtimeConnectionContext context, IRealtimeMessageSink sink, CancellationToken cancellationToken)
        {
            try
            {
                await Task.Delay(Timeout.Infinite, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                // Occupies the connection slot until the client disconnects.
            }
        }
    }

    public sealed class TestResyncHandler : IRealtimeResyncHandler
    {
        public RealtimeResyncRequest? Captured { get; private set; }
        public Task HandleAsync(RealtimeResyncRequest request, CancellationToken cancellationToken = default)
        {
            Captured = request;
            return Task.CompletedTask;
        }
    }

    public static WebApplication BuildApp(
        IRealtimeConnectionAuthorizer authorizer,
        IRealtimeSseSource source,
        IRealtimeCallerResolver? resolver = null,
        IRealtimeTenantRouter? router = null,
        Action<RealtimeConnectionOptions>? configure = null,
        bool withSignalR = false,
        IRealtimeResyncHandler? resyncHandler = null)
    {
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseTestServer();
        builder.Services.AddPlatformRealtimeAspNetCore(configure ?? (_ => { }));
        builder.Services.AddSingleton(authorizer);
        builder.Services.AddSingleton(source);
        if (resolver is not null) builder.Services.AddSingleton<IRealtimeCallerResolver>(resolver);
        if (router is not null) builder.Services.AddSingleton<IRealtimeTenantRouter>(router);
        if (resyncHandler is not null) builder.Services.AddSingleton<IRealtimeResyncHandler>(resyncHandler);
        if (withSignalR) builder.Services.AddPlatformRealtimeSignalR();

        var app = builder.Build();
        app.MapPlatformRealtimeSse("/sse");
        app.MapPlatformRealtimeStatus("/status");
        app.MapPlatformRealtimeResync("/resync");
        app.StartAsync().GetAwaiter().GetResult();
        return app;
    }
}
