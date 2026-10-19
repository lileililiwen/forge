using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Diagnostics.HealthChecks;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.Extensions.Options;
using Platform.AspNetCore;

namespace Platform.AspNetCore.HealthChecks;

/// <summary>
/// Helpers for registering and mapping platform health checks. The
/// package never assumes a database, queue, or external provider.
/// </summary>
public static class HealthCheckEndpointExtensions
{
    private static readonly string[] LiveTag = { "live" };

    /// <summary>
    /// Adds a minimal platform health check. The check always reports
    /// <see cref="HealthStatus.Healthy"/>; consumers register
    /// additional checks through the returned
    /// <see cref="IHealthChecksBuilder"/>.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The health checks builder for further configuration.</returns>
    public static IHealthChecksBuilder AddPlatformHealthChecks(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        return services.AddHealthChecks()
            .AddCheck("platform.liveness", () => HealthCheckResult.Healthy("Platform liveness"), tags: LiveTag);
    }

    /// <summary>
    /// Maps the platform health endpoint at the path configured in
    /// <see cref="PlatformAspNetCoreOptions.HealthCheckPath"/>. The
    /// endpoint returns plain text reflecting the aggregated status.
    /// </summary>
    /// <param name="endpoints">The endpoint route builder.</param>
    /// <returns>The endpoint convention builder.</returns>
    public static IEndpointConventionBuilder MapPlatformHealthEndpoint(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var options = endpoints.ServiceProvider.GetRequiredService<IOptions<PlatformAspNetCoreOptions>>().Value;
        return endpoints.MapHealthChecks(options.HealthCheckPath, new HealthCheckOptions
        {
            ResponseWriter = WriteResponse,
        });
    }

    private static Task WriteResponse(HttpContext context, HealthReport report)
    {
        context.Response.ContentType = "text/plain; charset=utf-8";
        return context.Response.WriteAsync(report.Status.ToString(), context.RequestAborted);
    }
}
