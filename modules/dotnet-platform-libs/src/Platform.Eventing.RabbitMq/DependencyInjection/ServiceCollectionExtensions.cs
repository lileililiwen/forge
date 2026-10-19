using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;

namespace Platform.Eventing.RabbitMq.DependencyInjection;

/// <summary>
/// Opt-in registration for the RabbitMQ durable eventing adapter. Every
/// mapping is added with a <c>TryAdd</c> so application-owned registrations —
/// including a consumer-owned <see cref="Platform.Eventing.Contracts.IDurableEventPublisher"/> —
/// always win.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers <see cref="RabbitMqEventingOptions"/> (validated at
    /// registration), the fail-closed default topology, the default channel
    /// factory, and the adapter as
    /// <see cref="Platform.Eventing.Contracts.IDurableEventPublisher"/> when
    /// the consumer has not registered its own implementation.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The optional configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    /// <exception cref="ArgumentException">The configured options are invalid.</exception>
    public static IServiceCollection AddPlatformRabbitMqEventing(
        this IServiceCollection services,
        Action<RabbitMqEventingOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        Action<RabbitMqEventingOptions> configureDelegate = configure ?? DefaultConfigure;
        var validated = new RabbitMqEventingOptions();
        configureDelegate(validated);
        validated.Validate();

        services
            .AddOptions<RabbitMqEventingOptions>()
            .Configure(configureDelegate);

        services.TryAddSingleton<IRabbitMqEventTopology, UnconfiguredRabbitMqEventTopology>();
        services.TryAddSingleton<IRabbitMqChannelFactory>(static sp =>
        {
            var options = sp.GetRequiredService<IOptions<RabbitMqEventingOptions>>().Value;
            options.Validate();
            return new RabbitMqChannelFactory(options);
        });
        services.TryAddSingleton<Platform.Eventing.Contracts.IDurableEventPublisher, RabbitMqDurableEventPublisher>();

        return services;
    }

    private static void DefaultConfigure(RabbitMqEventingOptions options)
    {
    }
}
