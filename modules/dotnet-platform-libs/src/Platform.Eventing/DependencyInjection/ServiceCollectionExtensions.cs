using System.Text.Json;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Eventing.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform eventing
/// package.
/// <see cref="AddPlatformEventing(IServiceCollection)"/> registers
/// the documented <see cref="IIntegrationEventEnvelopeSerializer"/>,
/// <see cref="IIntegrationEventEnvelopeDeserializer"/>, and
/// <see cref="EventingOptions"/>.
/// <see cref="AddPlatformEventingInProcess(IServiceCollection)"/>
/// additionally registers the documented
/// <see cref="InProcessEventBus"/> as the default
/// <see cref="IEventBus"/>.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the documented platform eventing contracts and
    /// configuration. The host is expected to call
    /// <see cref="AddPlatformEventingInProcess(IServiceCollection)"/>
    /// (or register their own <see cref="IEventBus"/>) to publish
    /// envelopes.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformEventing(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        return services.AddPlatformEventing(_ => { });
    }

    /// <summary>
    /// Registers the documented platform eventing contracts and
    /// applies the supplied <paramref name="configure"/> delegate.
    /// Consumers typically bind the
    /// <see cref="EventingOptions.SectionName"/> configuration
    /// section through
    /// <c>configuration.GetSection(...).Bind(options)</c> here.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="configure"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformEventing(
        this IServiceCollection services,
        Action<EventingOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<EventingOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());
        services.TryAddSingleton<IIntegrationEventEnvelopeSerializer>(sp =>
            new IntegrationEventEnvelopeSerializer(sp.GetRequiredService<IClock>()));
        services.TryAddSingleton<IIntegrationEventEnvelopeDeserializer>(sp =>
            new IntegrationEventEnvelopeDeserializer(sp.GetRequiredService<IClock>()));

        return services;
    }

    /// <summary>
    /// Registers the in-process <see cref="IEventBus"/> on top of
    /// <see cref="AddPlatformEventing(IServiceCollection)"/>. The bus
    /// is registered as a singleton; consumers should resolve it
    /// through <c>IServiceProvider</c> or inject it directly.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformEventingInProcess(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        services.TryAddSingleton<InProcessEventBus>(sp =>
        {
            var options = sp.GetRequiredService<IOptions<EventingOptions>>().Value;
            return new InProcessEventBus(
                sp.GetRequiredService<IClock>(),
                sp.GetRequiredService<IIntegrationEventEnvelopeDeserializer>(),
                sp,
                sp.GetRequiredService<ILogger<InProcessEventBus>>(),
                options.InProcessBoundedCapacity);
        });
        services.TryAddSingleton<IEventBus>(sp => sp.GetRequiredService<InProcessEventBus>());

        return services;
    }
}
