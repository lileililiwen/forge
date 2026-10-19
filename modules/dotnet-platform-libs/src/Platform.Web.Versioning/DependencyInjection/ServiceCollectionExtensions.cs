using Asp.Versioning;
using Asp.Versioning.ApiExplorer;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Mvc.ApiExplorer;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;

namespace Platform.Web.Versioning.DependencyInjection;

/// <summary>Opt-in registration, configuration, and pipeline helpers for the platform web versioning package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the platform web versioning defaults without changing application routing.</summary>
    public static IServiceCollection AddPlatformWebVersioning(this IServiceCollection services)
        => services.AddPlatformWebVersioning(_ => { });

    /// <summary>Registers the platform web versioning defaults and applies the supplied configuration.</summary>
    public static IServiceCollection AddPlatformWebVersioning(this IServiceCollection services, Action<PlatformWebVersioningOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services.AddValidatedOptions(configure);
        services.TryAddSingleton<IPlatformVersioningDefaultsProvider, PlatformVersioningDefaultsProvider>();
        services.AddOptions<ApiVersioningOptions>().Configure<IOptions<PlatformWebVersioningOptions>>(PlatformVersioningConfigurator.Configure);
        services.AddOptions<ApiExplorerOptions>().Configure<IOptions<PlatformWebVersioningOptions>>(PlatformVersioningConfigurator.ConfigureExplorer);
        var builder = services.AddApiVersioning();
        builder.AddApiExplorer();
        return services;
    }

    /// <summary>
    /// Enables the platform <see cref="ApiVersion"/> model binding in minimal API parameter lists. The
    /// platform keeps this opt-in so consumers that do not yet bind <see cref="ApiVersion"/> in
    /// route handlers are not affected.
    /// </summary>
    public static IApiVersioningBuilder EnablePlatformApiVersionBinding(this IApiVersioningBuilder builder)
    {
        ArgumentNullException.ThrowIfNull(builder);
        return builder.EnableApiVersionBinding();
    }

    /// <summary>
    /// Maps every API description exposed by the platform API Explorer as a routed endpoint, using
    /// the supplied <paramref name="configure"/> to add application-specific conventions. The
    /// platform does not own OpenAPI generation; this helper only routes the descriptions the
    /// application has already registered (typically via the <c>Platform.Web.OpenApi</c> package).
    /// </summary>
    public static IEndpointRouteBuilder MapPlatformApiExplorerDescriptions(
        this IEndpointRouteBuilder endpoints,
        Action<IEndpointConventionBuilder, ApiVersionDescription> configure)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        ArgumentNullException.ThrowIfNull(configure);
        var provider = endpoints.ServiceProvider.GetService<IApiVersionDescriptionProvider>();
        if (provider is null)
        {
            throw new InvalidOperationException(
                "API version descriptions are not registered. Call AddPlatformWebVersioning before mapping explorer descriptions.");
        }
        var groups = provider.ApiVersionDescriptions
            .GroupBy(description => description.GroupName ?? description.ApiVersion.ToString("'v'VVV", System.Globalization.CultureInfo.InvariantCulture));
        foreach (var group in groups)
        {
            var builder = endpoints.MapGroup($"/{group.Key}");
            foreach (var description in group)
            {
                configure(builder, description);
            }
        }
        return endpoints;
    }

    private static IServiceCollection AddValidatedOptions(this IServiceCollection services, Action<PlatformWebVersioningOptions> configure)
    {
        services.AddOptions<PlatformWebVersioningOptions>()
            .Configure(configure)
            .Validate(o => o.Validate().Count == 0, "Platform web versioning options are invalid.");
        return services;
    }
}
