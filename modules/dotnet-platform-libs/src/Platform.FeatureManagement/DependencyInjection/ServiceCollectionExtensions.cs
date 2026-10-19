using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.FeatureManagement;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.FeatureManagement.DependencyInjection;

/// <summary>Registration helpers for the platform feature-management package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers feature management with the platform tenant filter and a replaceable context resolver. Reads feature definitions from the configured section.</summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configuration">The application configuration (owns the feature definition section).</param>
    /// <returns>The same service collection for chaining.</returns>
    public static IServiceCollection AddPlatformFeatureManagement(this IServiceCollection services, IConfiguration configuration) =>
        services.AddPlatformFeatureManagement(configuration, _ => { });

    /// <summary>Registers feature management with the platform tenant filter, a replaceable context resolver, and the supplied options.</summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configuration">The application configuration (owns the feature definition section).</param>
    /// <param name="configure">The platform feature-management options configuration delegate.</param>
    /// <returns>The same service collection for chaining.</returns>
    public static IServiceCollection AddPlatformFeatureManagement(this IServiceCollection services, IConfiguration configuration, Action<FeatureManagementOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configuration);
        ArgumentNullException.ThrowIfNull(configure);

        services.AddPlatformWebTelemetry();
        services.AddValidatedOptions(configure);

        var options = new FeatureManagementOptions();
        configure(options);

        if (options.Enabled)
        {
            var section = configuration.GetSection(options.SectionName);
            services.AddFeatureManagement(section)
                .AddFeatureFilter<PlatformTenantFeatureFilter>();
        }

        services.TryAddSingleton<IFeatureContextResolver, DefaultFeatureContextResolver>();
        return services;
    }
}
