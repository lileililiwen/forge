using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;

namespace Platform.Web.Composition;

/// <summary>
/// Opt-in pipeline integration for explicitly registered modules. When the
/// extensions are not called, no module middleware or endpoints are added.
/// Each invocation runs every registered module hook at most once, in
/// registry order.
/// </summary>
public static class PlatformWebModuleApplicationBuilderExtensions
{
    /// <summary>
    /// Invokes <see cref="IPlatformWebModule.ConfigureMiddleware"/> for each
    /// registered module, in registry order. Opt-in: modules registered
    /// without this call contribute no middleware.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The application builder.</returns>
    public static IApplicationBuilder UsePlatformWebModules(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        var registry = PlatformWebModuleRegistry.FromProvider(app.ApplicationServices);
        foreach (var module in registry.Modules)
        {
            module.ConfigureMiddleware(app);
        }

        return app;
    }

    /// <summary>
    /// Invokes <see cref="IPlatformWebModule.MapEndpoints"/> for each
    /// registered module, in registry order. Opt-in: modules registered
    /// without this call map no endpoints.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <returns>The endpoint route builder.</returns>
    public static IEndpointRouteBuilder MapPlatformWebModules(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var registry = PlatformWebModuleRegistry.FromProvider(endpoints.ServiceProvider);
        foreach (var module in registry.Modules)
        {
            module.MapEndpoints(endpoints);
        }

        return endpoints;
    }
}
