using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Jobs.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform scheduling
/// package. <see cref="AddPlatformJobs(IServiceCollection)"/> binds
/// <see cref="BackgroundJobsOptions"/> to the documented
/// <see cref="BackgroundJobsOptions.SectionName"/> configuration
/// section and registers <see cref="IClock"/> when no implementation
/// is already present. The package does NOT register default
/// implementations of <see cref="IJobDispatcher"/>,
/// <see cref="IRecurringJobRegistry"/>, or <see cref="IJobTelemetry"/>;
/// consumers provide their own.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform scheduling options with the documented
    /// defaults applied. The consumer is expected to either call the
    /// <see cref="AddPlatformJobs(IServiceCollection, Action{BackgroundJobsOptions})"/>
    /// overload with a configuration-binding delegate, or to read
    /// <c>IConfiguration</c> from the host and invoke the
    /// <see cref="BackgroundJobsOptions.SectionName"/> section through
    /// <c>IServiceCollection.Configure&lt;BackgroundJobsOptions&gt;</c>.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformJobs(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        return services.AddPlatformJobs(_ => { });
    }

    /// <summary>
    /// Registers the platform scheduling options and applies the
    /// supplied <paramref name="configure"/> delegate. The delegate
    /// is invoked once with a fresh <see cref="BackgroundJobsOptions"/>
    /// instance carrying the documented defaults; consumers typically
    /// bind the <see cref="BackgroundJobsOptions.SectionName"/>
    /// configuration section through
    /// <c>configuration.GetSection(...).Bind(options)</c> here.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="configure"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformJobs(
        this IServiceCollection services,
        Action<BackgroundJobsOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<BackgroundJobsOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());

        return services;
    }
}
