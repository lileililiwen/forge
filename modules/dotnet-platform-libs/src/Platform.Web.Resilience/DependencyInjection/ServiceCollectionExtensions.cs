using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Web.Resilience.Handlers;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Web.Resilience.DependencyInjection;

/// <summary>Registration helpers for the platform HTTP resilience package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the default platform HTTP resilience handler and telemetry with default options.</summary>
    public static IServiceCollection AddPlatformHttpResilience(this IServiceCollection services) => services.AddPlatformHttpResilience(_ => { });

    /// <summary>Registers the default platform HTTP resilience handler and telemetry and configures the supplied options.</summary>
    public static IServiceCollection AddPlatformHttpResilience(this IServiceCollection services, Action<PlatformHttpResilienceOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddPlatformWebTelemetry();
        services.AddValidatedOptions(configure);
        services.TryAddSingleton<IHttpResilienceTelemetry, DefaultHttpResilienceTelemetry>();
        return services;
    }

    /// <summary>Adds the platform HTTP resilience handler to the named <see cref="HttpClient"/> registration.</summary>
    public static IHttpClientBuilder AddPlatformHttpResilience(this IHttpClientBuilder builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        builder.Services.AddPlatformHttpResilience();
        return builder.AddHttpMessageHandler<PlatformHttpResilienceHandler>();
    }
}
