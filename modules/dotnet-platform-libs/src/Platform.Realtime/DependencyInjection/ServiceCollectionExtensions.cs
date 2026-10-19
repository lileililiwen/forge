using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Realtime.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the transport-neutral realtime
/// contracts. <see cref="AddPlatformRealtime(IServiceCollection)"/> binds
/// <see cref="RealtimeConnectionOptions"/> to the documented
/// <see cref="RealtimeConnectionOptions.SectionName"/> section and registers
/// <see cref="IClock"/> when no implementation is already present. The package
/// does NOT register the application-owned <see cref="Authorization.IRealtimeConnectionAuthorizer"/>
/// or <see cref="Tenant.IRealtimeTenantRouter"/>; the ASP.NET Core adapter
/// (or the host) must supply them so connections fail closed by default.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the realtime contracts with default options.</summary>
    public static IServiceCollection AddPlatformRealtime(this IServiceCollection services) =>
        services.AddPlatformRealtime(_ => { });

    /// <summary>Registers the realtime contracts and applies the supplied options.</summary>
    public static IServiceCollection AddPlatformRealtime(
        this IServiceCollection services,
        Action<RealtimeConnectionOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<RealtimeConnectionOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());

        return services;
    }
}
