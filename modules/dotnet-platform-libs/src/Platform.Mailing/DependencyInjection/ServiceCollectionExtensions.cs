using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Mailing.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform mailing package.
/// <see cref="AddPlatformMailing(IServiceCollection)"/> binds
/// <see cref="MailingOptions"/> to the documented
/// <see cref="MailingOptions.SectionName"/> configuration section and
/// registers <see cref="IClock"/> when no implementation is already
/// present. The package does NOT register default implementations of
/// <see cref="IMailService"/> or <see cref="IMailTemplateRenderer{TModel}"/>;
/// consumers provide their own.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform mailing options with the documented
    /// defaults applied. The consumer is expected to either call the
    /// <see cref="AddPlatformMailing(IServiceCollection, Action{MailingOptions})"/>
    /// overload with a configuration-binding delegate, or to read
    /// <c>IConfiguration</c> from the host and invoke the
    /// <see cref="MailingOptions.SectionName"/> section through
    /// <c>IServiceCollection.Configure&lt;MailingOptions&gt;</c>.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformMailing(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        return services.AddPlatformMailing(_ => { });
    }

    /// <summary>
    /// Registers the platform mailing options and applies the supplied
    /// <paramref name="configure"/> delegate. The delegate is invoked
    /// once with a fresh <see cref="MailingOptions"/> instance carrying
    /// the documented defaults; consumers typically bind the
    /// <see cref="MailingOptions.SectionName"/> configuration section
    /// through
    /// <c>configuration.GetSection(...).Bind(options)</c> here.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="configure"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformMailing(
        this IServiceCollection services,
        Action<MailingOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<MailingOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());

        return services;
    }
}
