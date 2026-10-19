using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.RateLimiting.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform rate-limiting
/// package.
/// <see cref="AddPlatformRateLimiting(IServiceCollection)"/> registers
/// the documented <see cref="InMemoryRateLimiter"/>,
/// <see cref="IRateLimitBypassResolver"/>, and
/// <see cref="IRateLimiterBackendStatusProvider"/> on top of the
/// <see cref="RateLimitingOptions"/> configuration type. Consumers can
/// override the documented default
/// <see cref="RateLimitPolicies"/> catalog by registering their own
/// <see cref="RateLimitPolicies"/> before calling
/// <see cref="AddPlatformRateLimiting(IServiceCollection)"/>.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform rate-limiting services with the
    /// documented defaults applied. The consumer is expected to
    /// either call the
    /// <see cref="AddPlatformRateLimiting(IServiceCollection, Action{RateLimitingOptions})"/>
    /// overload with a configuration-binding delegate, or to read
    /// <c>IConfiguration</c> from the host and invoke the
    /// <see cref="RateLimitingOptions.SectionName"/> section through
    /// <c>IServiceCollection.Configure&lt;RateLimitingOptions&gt;</c>.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformRateLimiting(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        return services.AddPlatformRateLimiting(_ => { });
    }

    /// <summary>
    /// Registers the platform rate-limiting services and applies the
    /// supplied <paramref name="configure"/> delegate. The delegate
    /// is invoked once with a fresh <see cref="RateLimitingOptions"/>
    /// instance carrying the documented defaults; consumers typically
    /// bind the <see cref="RateLimitingOptions.SectionName"/>
    /// configuration section through
    /// <c>configuration.GetSection(...).Bind(options)</c> here.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="configure"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformRateLimiting(
        this IServiceCollection services,
        Action<RateLimitingOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<RateLimitingOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());
        services.TryAddSingleton<IRateLimiter, InMemoryRateLimiter>();
        services.TryAddSingleton<IRateLimitBypassResolver, ConfigurationRateLimitBypassResolver>();
        services.TryAddSingleton<IRateLimiterBackendStatusProvider, InMemoryRateLimiterBackendStatusProvider>();

        return services;
    }
}
