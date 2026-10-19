using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Realtime;
using Platform.Realtime.AspNetCore.Caller;
using Platform.Realtime.AspNetCore.Status;
using Platform.Realtime.Authorization;
using Platform.Realtime.DependencyInjection;
using Platform.Realtime.Status;
using Platform.Realtime.Tenant;

namespace Platform.Realtime.AspNetCore.Sse;

/// <summary>
/// Registration extensions for the ASP.NET Core realtime adapters. The shared
/// registration wires the transport-neutral contracts, the fail-closed defaults,
/// and the SSE connection limiter. SignalR registration builds on top of it.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform realtime ASP.NET Core adapters with default options.
    /// </summary>
    public static IServiceCollection AddPlatformRealtimeAspNetCore(this IServiceCollection services) =>
        services.AddPlatformRealtimeAspNetCore(_ => { });

    /// <summary>
    /// Registers the platform realtime ASP.NET Core adapters and configures the
    /// shared connection options.
    /// </summary>
    public static IServiceCollection AddPlatformRealtimeAspNetCore(
        this IServiceCollection services,
        Action<RealtimeConnectionOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services.AddPlatformRealtime(configure);

        services.TryAddSingleton<IRealtimeConnectionAuthorizer, DenyAllRealtimeAuthorizer>();
        services.TryAddSingleton<IRealtimeTenantRouter, DenyCrossTenantRouter>();
        services.TryAddSingleton<IRealtimeCallerResolver, AnonymousRealtimeCallerResolver>();
        services.TryAddSingleton<RealtimeConnectionLimiter>();
        services.TryAddSingleton<IRealtimeProviderStatus>(_ => new CompositeRealtimeProviderStatus(signalREnabled: false, backplaneEnabled: false));

        return services;
    }
}
