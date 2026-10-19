using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;

namespace Platform.Web.Composition;

/// <summary>
/// Explicit application module seam for ASP.NET Core composition. A module
/// declares a stable <see cref="Name"/>, a deterministic <see cref="Order"/>,
/// and hooks for service registration, middleware, and endpoint mapping.
/// The platform never scans assemblies, instantiates unregistered types, or
/// registers Mediator, validators, persistence, or product services.
/// </summary>
public interface IPlatformWebModule
{
    /// <summary>
    /// Gets the stable module name. Names must be unique across the
    /// registered module set; registration rejects duplicates.
    /// </summary>
    string Name { get; }

    /// <summary>
    /// Gets the declared order. Modules run ordered by <see cref="Order"/>
    /// and then by <see cref="Name"/> (ordinal).
    /// </summary>
    int Order { get; }

    /// <summary>
    /// Registers the module services. Called once per explicit
    /// <c>AddPlatformWebModule</c> registration.
    /// </summary>
    /// <param name="services">The service collection.</param>
    void ConfigureServices(IServiceCollection services);

    /// <summary>
    /// Adds the module middleware. Called once per
    /// <c>UsePlatformWebModules</c> invocation, in module order.
    /// The default implementation is a no-op so modules that only
    /// contribute services or endpoints are not forced to add middleware.
    /// </summary>
    /// <param name="app">The application builder.</param>
    void ConfigureMiddleware(IApplicationBuilder app)
    {
    }

    /// <summary>
    /// Maps the module endpoints. Called once per
    /// <c>MapPlatformWebModules</c> invocation, in module order.
    /// The default implementation is a no-op so modules that only
    /// contribute services or middleware are not forced to map endpoints.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    void MapEndpoints(IEndpointRouteBuilder endpoints)
    {
    }
}
