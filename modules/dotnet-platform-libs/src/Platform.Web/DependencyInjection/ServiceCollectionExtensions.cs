using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.AspNetCore.DependencyInjection;
using Platform.AspNetCore.HealthChecks;
using Microsoft.Extensions.Diagnostics.HealthChecks;

namespace Platform.Web.DependencyInjection;

/// <summary>Explicit registration and pipeline extensions for Platform.Web.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the composable web runtime with safe defaults.</summary>
    public static IServiceCollection AddPlatformWeb(this IServiceCollection services) => services.AddPlatformWeb(_ => { });

    /// <summary>Registers the composable web runtime and configures its options.</summary>
    public static IServiceCollection AddPlatformWeb(this IServiceCollection services, Action<PlatformWebOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<PlatformWebOptions>().Configure(configure).Validate(o => o.Validate().Count == 0, "Platform web options are invalid.");
        services.AddPlatformAspNetCore(o => { });
        services.AddOptions<Platform.AspNetCore.PlatformAspNetCoreOptions>().Configure<IOptions<PlatformWebOptions>>((asp, web) =>
        {
            asp.CorrelationHeader = web.Value.CorrelationHeader;
            asp.AcceptIncomingCorrelationHeader = web.Value.AcceptIncomingCorrelationHeader;
            asp.MaxCorrelationIdLength = web.Value.MaxCorrelationIdLength;
        });
        services.TryAddSingleton<IPlatformRedactor, DefaultPlatformRedactor>();
        services.TryAddSingleton<IPlatformConfigurationValidator, DefaultPlatformConfigurationValidator>();
        services.TryAddSingleton<IProviderStatusSource, EmptyProviderStatusSource>();
        services.AddPlatformHealthChecks();
        return services;
    }

    /// <summary>Adds correlation, safe errors, security headers, limits, and timeout middleware.</summary>
    public static IApplicationBuilder UsePlatformWeb(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        return app.UsePlatformAspNetCore()
            .UseMiddleware<PlatformSecurityHeadersMiddleware>()
            .UseMiddleware<PlatformRequestLimitsMiddleware>()
            .UseMiddleware<PlatformRequestTimeoutMiddleware>();
    }

    /// <summary>Maps independent liveness and readiness endpoints.</summary>
    public static IEndpointRouteBuilder MapPlatformRuntimeEndpoints(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var options = endpoints.ServiceProvider.GetRequiredService<IOptions<PlatformWebOptions>>().Value;
        endpoints.MapGet(options.LivePath, () => Results.Json(new { status = "live" }));
        endpoints.MapGet(options.ReadinessPath, async (HealthCheckService health, IProviderStatusSource providers, HttpContext context) =>
        {
            var report = await health.CheckHealthAsync(c => !c.Tags.Contains("live"), context.RequestAborted);
            var statuses = providers.GetStatuses();
            var ready = report.Status == Microsoft.Extensions.Diagnostics.HealthChecks.HealthStatus.Healthy
                && statuses.All(s => s.Available);
            var result = new
            {
                status = ready ? "ready" : "not_ready",
                checks = report.Entries.ToDictionary(e => e.Key, e => e.Value.Status.ToString()),
                providers = statuses.Select(s => new { name = s.Name, available = s.Available }).ToArray(),
            };
            return Results.Json(result, statusCode: ready ? StatusCodes.Status200OK : StatusCodes.Status503ServiceUnavailable);
        });
        return endpoints;
    }
}
