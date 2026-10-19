using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Mailing;
using SendGrid;

namespace Platform.Mailing.SendGrid.DependencyInjection;

/// <summary>
/// Opt-in registration for the SendGrid mail adapter. Both the
/// <see cref="ISendGridClient"/> and the <see cref="IMailService"/> mappings are
/// added with a <c>TryAdd</c> so application-owned registrations made before
/// this call always win.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers <see cref="SendGridMailOptions"/> (validated at
    /// registration), maps a default <see cref="ISendGridClient"/> when the
    /// consumer has not registered one, and maps the adapter to
    /// <see cref="IMailService"/> when the consumer has not registered its
    /// own implementation.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The optional configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The configured options are invalid.</exception>
    public static IServiceCollection AddPlatformSendGridMail(
        this IServiceCollection services,
        Action<SendGridMailOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        Action<SendGridMailOptions> configureDelegate = configure ?? DefaultConfigure;
        var validated = new SendGridMailOptions();
        configureDelegate(validated);
        validated.Validate();

        services
            .AddOptions<SendGridMailOptions>()
            .Configure(configureDelegate);

        services.TryAddSingleton<ISendGridClient>(static sp =>
        {
            var options = sp.GetRequiredService<IOptions<SendGridMailOptions>>().Value;
            options.Validate();
            return new SendGridClient(new SendGridClientOptions
            {
                ApiKey = options.ApiKey,
                HttpErrorAsException = false,
            });
        });

        services.TryAddSingleton<IMailService, SendGridMailService>();

        return services;
    }

    private static void DefaultConfigure(SendGridMailOptions options)
    {
    }
}
