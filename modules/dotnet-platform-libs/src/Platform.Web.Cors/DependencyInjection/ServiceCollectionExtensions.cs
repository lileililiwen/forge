using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Cors.Infrastructure;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Platform.Web.Telemetry;
using Platform.Web.Telemetry.DependencyInjection;

namespace Platform.Web.Cors.DependencyInjection;

/// <summary>Registration and pipeline extensions for the platform CORS package.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the platform CORS policies with default options.</summary>
    public static IServiceCollection AddPlatformWebCors(this IServiceCollection services) => services.AddPlatformWebCors(_ => { });

    /// <summary>Registers the platform CORS policies and configures the supplied options.</summary>
    public static IServiceCollection AddPlatformWebCors(this IServiceCollection services, Action<PlatformWebCorsOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddPlatformWebTelemetry();
        services.AddValidatedOptions(configure);
        services.AddCors(cors =>
        {
            var sp = services.BuildServiceProvider();
            var options = sp.GetRequiredService<IOptions<PlatformWebCorsOptions>>().Value;
            foreach (var policy in options.Policies)
            {
                cors.AddPolicy(policy.Name, builder => Apply(builder, policy));
            }
        });
        return services;
    }

    /// <summary>Configures the platform CORS policies after the host environment is known.</summary>
    public static IServiceCollection AddPlatformWebCors(this IServiceCollection services, IHostEnvironment environment, Action<PlatformWebCorsOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(environment);
        return services.AddPlatformWebCors(options =>
        {
            options.Environment ??= environment.EnvironmentName;
            configure(options);
        });
    }

    /// <summary>Installs the CORS middleware using the policy named <c>default</c>.</summary>
    public static IApplicationBuilder UsePlatformWebCors(this IApplicationBuilder app) => app.UsePlatformWebCors("default");

    /// <summary>Installs the CORS middleware using the named policy.</summary>
    public static IApplicationBuilder UsePlatformWebCors(this IApplicationBuilder app, string policyName)
    {
        ArgumentNullException.ThrowIfNull(app);
        if (string.IsNullOrWhiteSpace(policyName)) throw new ArgumentException("A CORS policy name is required.", nameof(policyName));
        return app.UseCors(policyName);
    }

    private static void Apply(CorsPolicyBuilder builder, PlatformWebCorsPolicyOptions policy)
    {
        if (policy.AllowedOrigins.Count == 1 && string.Equals(policy.AllowedOrigins[0], "*", StringComparison.Ordinal))
        {
            builder.AllowAnyOrigin();
        }
        else
        {
            builder.WithOrigins(policy.AllowedOrigins.ToArray());
        }

        if (policy.AllowedHeaders.Count == 1 && string.Equals(policy.AllowedHeaders[0], "*", StringComparison.OrdinalIgnoreCase))
            builder.AllowAnyHeader();
        else if (policy.AllowedHeaders.Count > 0)
            builder.WithHeaders(policy.AllowedHeaders.ToArray());

        if (policy.AllowedMethods.Count == 1 && string.Equals(policy.AllowedMethods[0], "*", StringComparison.OrdinalIgnoreCase))
            builder.AllowAnyMethod();
        else if (policy.AllowedMethods.Count > 0)
            builder.WithMethods(policy.AllowedMethods.ToArray());

        if (policy.ExposedHeaders.Count > 0) builder.WithExposedHeaders(policy.ExposedHeaders.ToArray());
        if (policy.AllowCredentials) builder.AllowCredentials();
        if (policy.PreflightMaxAge > TimeSpan.Zero) builder.SetPreflightMaxAge(policy.PreflightMaxAge);
    }
}
