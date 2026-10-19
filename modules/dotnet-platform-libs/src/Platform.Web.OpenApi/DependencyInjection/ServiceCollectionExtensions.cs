using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Web.OpenApi.DependencyInjection;

/// <summary>Registration and endpoint mapping helpers for the platform OpenAPI package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the platform OpenAPI document registry with default options.</summary>
    public static IServiceCollection AddPlatformWebOpenApi(this IServiceCollection services) => services.AddPlatformWebOpenApi(_ => { });

    /// <summary>Registers the platform OpenAPI document registry and configures the supplied options.</summary>
    public static IServiceCollection AddPlatformWebOpenApi(this IServiceCollection services, Action<PlatformWebOpenApiOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddPlatformWebTelemetry();
        services.AddValidatedOptions(configure);
        services.AddSingleton<IPlatformOpenApiDocumentRegistry, PlatformOpenApiDocumentRegistry>();
        return services;
    }

    /// <summary>Registers a static OpenAPI document provider with the supplied options.</summary>
    public static IServiceCollection AddPlatformOpenApiDocument(this IServiceCollection services, PlatformWebOpenApiDocumentOptions document)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(document);
        services.AddPlatformWebOpenApi();
        services.AddSingleton<IPlatformOpenApiDocumentProvider>(_ => new Providers.StaticPlatformOpenApiDocumentProvider(document));
        return services;
    }

    /// <summary>Maps a single named OpenAPI document at the configured path. Authorization metadata is preserved.</summary>
    public static IEndpointConventionBuilder MapPlatformOpenApiDocument(this IEndpointRouteBuilder endpoints, string name)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        if (string.IsNullOrWhiteSpace(name)) throw new ArgumentException("A document name is required.", nameof(name));
        return endpoints.MapGet($"/openapi/{name}.json", (HttpContext context) =>
        {
            var registry = context.RequestServices.GetRequiredService<IPlatformOpenApiDocumentRegistry>();
            var document = registry.Resolve(name);
            if (document is null)
            {
                context.Response.StatusCode = StatusCodes.Status404NotFound;
                return context.Response.WriteAsync($"Document '{name}' is not registered.", context.RequestAborted);
            }
            context.Response.StatusCode = StatusCodes.Status200OK;
            context.Response.ContentType = "application/json; charset=utf-8";
            return context.Response.WriteAsync(document, context.RequestAborted);
        });
    }

    /// <summary>Maps every registered OpenAPI document at its configured path.</summary>
    public static IEndpointRouteBuilder MapPlatformOpenApiDocuments(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var registry = endpoints.ServiceProvider.GetRequiredService<IPlatformOpenApiDocumentRegistry>();
        foreach (var name in registry.AvailableDocuments)
        {
            endpoints.MapPlatformOpenApiDocument(name);
        }
        return endpoints;
    }
}
