using Microsoft.AspNetCore.SignalR;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Realtime;
using Platform.Realtime.AspNetCore.Sse;
using Platform.Realtime.AspNetCore.Status;
using Platform.Realtime.Status;

namespace Platform.Realtime.AspNetCore.SignalR;

/// <summary>
/// Registration extensions for the platform SignalR realtime adapter.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform SignalR pipeline: the ASP.NET Core shared adapters,
    /// the SignalR server, the connection-authorization filter, and a provider
    /// status that reflects the optional backplane. The backplane is
    /// application-owned; supply <see cref="RealtimeSignalROptions.ConfigureBackplane"/>
    /// to enable distributed scale-out, otherwise the host runs in-process.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">An optional delegate that configures SignalR options and the backplane seam.</param>
    /// <returns>The same service collection for chaining.</returns>
    public static IServiceCollection AddPlatformRealtimeSignalR(
        this IServiceCollection services,
        Action<RealtimeSignalROptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        services.AddPlatformRealtimeAspNetCore();

        var options = new RealtimeSignalROptions();
        configure?.Invoke(options);

        var signalR = services.AddSignalR();
        services.TryAddSingleton<IHubFilter, RealtimeHubAuthorizationFilter>();
        options.ConfigureBackplane?.Invoke(signalR);

        services.RemoveAll<IRealtimeProviderStatus>();
        services.AddSingleton<IRealtimeProviderStatus>(
            _ => new CompositeRealtimeProviderStatus(
                signalREnabled: true,
                backplaneEnabled: options.ConfigureBackplane is not null));

        return services;
    }
}
