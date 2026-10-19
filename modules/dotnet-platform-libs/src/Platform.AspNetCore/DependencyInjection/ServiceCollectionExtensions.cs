using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.AspNetCore.Correlation;
using Platform.AspNetCore.Errors;
using Platform.AspNetCore.HealthChecks;
using Platform.Core.Time;
using IClock = Platform.Core.Time.IClock;
using SystemClock = Platform.Core.Time.SystemClock;

namespace Platform.AspNetCore.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform ASP.NET Core
/// integration. The methods are explicit: each one registers only the
/// services it documents.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the core platform services (clock, problem-details
    /// mapper, HTTP context accessor). Use this overload when the
    /// host does not need an <see cref="IConfiguration"/> binding.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAspNetCore(this IServiceCollection services)
    {
        return services.AddPlatformAspNetCore(_ => { });
    }

    /// <summary>
    /// Registers the core platform services and configures the
    /// supplied <paramref name="configure"/> delegate.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    public static IServiceCollection AddPlatformAspNetCore(
        this IServiceCollection services,
        Action<PlatformAspNetCoreOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services.AddOptions<PlatformAspNetCoreOptions>().Configure(configure);
        services.TryAddSingleton<SystemClock>();
        services.TryAddSingleton<IClock>(sp => sp.GetRequiredService<SystemClock>());
        services.TryAddSingleton<IProblemDetailsMapper, PlatformProblemDetailsMapper>();
        services.TryAddSingleton<IHttpContextAccessor, HttpContextAccessor>();
        services.TryAddScoped<ICorrelationAccessor, HttpCorrelationAccessor>();

        return services;
    }

    /// <summary>
    /// Adds the platform ASP.NET Core middleware to the pipeline. The
    /// documented order is: <see cref="UsePlatformCorrelation"/>
    /// first, then <see cref="UsePlatformProblemDetails"/>. Health
    /// endpoints are added by
    /// <see cref="MapPlatformEndpoints"/>.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    public static IApplicationBuilder UsePlatformAspNetCore(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UsePlatformCorrelation().UsePlatformProblemDetails();
    }

    /// <summary>
    /// Adds the platform correlation middleware. The middleware reads
    /// or generates a correlation identifier and stores it on the
    /// current <see cref="HttpContext"/>.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    public static IApplicationBuilder UsePlatformCorrelation(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<CorrelationMiddleware>();
    }

    /// <summary>
    /// Adds the platform ProblemDetails exception middleware. Place
    /// this after <see cref="UsePlatformCorrelation"/> so correlation
    /// identifiers are available on error responses.
    /// </summary>
    /// <param name="app">The application builder.</param>
    /// <returns>The same <paramref name="app"/> for chaining.</returns>
    public static IApplicationBuilder UsePlatformProblemDetails(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UseMiddleware<ProblemDetailsExceptionMiddleware>();
    }

    /// <summary>
    /// Maps the platform health endpoint at the configured path.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <returns>The same <paramref name="endpoints"/> for chaining.</returns>
    public static IEndpointRouteBuilder MapPlatformEndpoints(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        endpoints.MapPlatformHealthEndpoint();
        return endpoints;
    }
}
