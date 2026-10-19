using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Mailing;

namespace Platform.Mailing.Smtp.DependencyInjection;

/// <summary>
/// Opt-in registration for the SMTP mail adapter. The registration never
/// overwrites an application-owned <see cref="IMailService"/>: the adapter is
/// added with a <c>TryAdd</c> so a consumer registration made before this call
/// always wins.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers <see cref="SmtpMailOptions"/> (validated at registration)
    /// and maps the adapter to <see cref="IMailService"/> when the consumer
    /// has not registered its own implementation.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The optional configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The configured options are invalid.</exception>
    public static IServiceCollection AddPlatformSmtpMail(
        this IServiceCollection services,
        Action<SmtpMailOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        Action<SmtpMailOptions> configureDelegate = configure ?? DefaultConfigure;
        var validated = new SmtpMailOptions();
        configureDelegate(validated);
        validated.Validate();

        services
            .AddOptions<SmtpMailOptions>()
            .Configure(configureDelegate);

        services.TryAddSingleton<IMailService, SmtpMailService>();

        return services;
    }

    private static void DefaultConfigure(SmtpMailOptions options)
    {
    }
}
