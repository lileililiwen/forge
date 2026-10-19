using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.Observability.Diagnostics;
using Platform.Observability.Hosting;
using Platform.Observability.Redaction;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Observability.DependencyInjection;

/// <summary>Registration helpers for the platform observability package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the platform observability pipeline with default options.</summary>
    public static IServiceCollection AddPlatformObservability(this IServiceCollection services) => services.AddPlatformObservability(_ => { });

    /// <summary>Registers the platform observability pipeline and configures its options.</summary>
    public static IServiceCollection AddPlatformObservability(this IServiceCollection services, Action<PlatformObservabilityOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddValidatedOptions(configure);
        services.TryAddSingleton<IPlatformObservabilityRedactor, DefaultPlatformObservabilityRedactor>();
        services.TryAddSingleton<ICorrelationIdGenerator, DefaultCorrelationIdGenerator>();
        services.TryAddSingleton<IPlatformActivityRecorder, DefaultPlatformActivityRecorder>();
        services.TryAddSingleton<IPlatformObservabilityProviderStatusSource, DefaultPlatformObservabilityProviderStatusSource>();
        services.TryAddSingleton<IPlatformObservabilityProviderRecorder, DefaultPlatformObservabilityProviderRecorder>();
        services.AddPlatformWebTelemetry();
        services.AddSingleton<IPlatformCorrelationAccessor>(sp => new HttpPlatformCorrelationAccessor(sp.GetRequiredService<IHttpContextAccessor>()));
        services.AddHttpContextAccessor();
        services.AddHostedService<PlatformObservabilityHostLifetime>();
        return services;
    }
}

/// <summary>Pipeline helpers for the platform observability package.</summary>
public static class ApplicationBuilderExtensions
{
    /// <summary>Adds the correlation middleware to the request pipeline.</summary>
    public static IApplicationBuilder UsePlatformObservability(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<PlatformObservabilityCorrelationMiddleware>();
    }
}
