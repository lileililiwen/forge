using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.Idempotency.DependencyInjection;

/// <summary>
/// Service-collection extensions that wire the platform idempotency
/// package. <see cref="AddPlatformIdempotency(IServiceCollection)"/>
/// registers <see cref="IIdempotencyStore"/> as the documented
/// <see cref="InMemoryIdempotencyStore"/>, binds
/// <see cref="IdempotencyOptions"/> to the
/// <see cref="IdempotencyOptions.SectionName"/> configuration section,
/// and registers <see cref="IClock"/> when no implementation is
/// already present. When <see cref="IdempotencyOptions.Enabled"/> is
/// <c>false</c> the registration is a no-op.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the platform idempotency services with the
    /// documented defaults applied. The consumer is expected to
    /// either call the
    /// <see cref="AddPlatformIdempotency(IServiceCollection, Action{IdempotencyOptions})"/>
    /// overload with a configuration-binding delegate, or to read
    /// <c>IConfiguration</c> from the host and invoke the
    /// <see cref="IdempotencyOptions.SectionName"/> section through
    /// <c>IServiceCollection.Configure&lt;IdempotencyOptions&gt;</c>.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformIdempotency(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);

        return services.AddPlatformIdempotency(_ => { });
    }

    /// <summary>
    /// Registers the platform idempotency services and applies the
    /// supplied <paramref name="configure"/> delegate. The delegate
    /// is invoked once with a fresh <see cref="IdempotencyOptions"/>
    /// instance carrying the documented defaults; consumers typically
    /// bind the <see cref="IdempotencyOptions.SectionName"/>
    /// configuration section through
    /// <c>configuration.GetSection(...).Bind(options)</c> here.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">The options configuration delegate.</param>
    /// <returns>The same <paramref name="services"/> for chaining.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="services"/> or <paramref name="configure"/> is <c>null</c>.</exception>
    public static IServiceCollection AddPlatformIdempotency(
        this IServiceCollection services,
        Action<IdempotencyOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        services
            .AddOptions<IdempotencyOptions>()
            .Configure(configure);

        services.TryAddSingleton<IClock>(_ => new SystemClock());

        // Resolve the configured options through the post-configure
        // snapshot so the Enabled flag governs the store registration.
        services.TryAddSingleton<IIdempotencyStore>(sp =>
        {
            var options = sp.GetRequiredService<IOptions<IdempotencyOptions>>().Value;
            if (!options.Enabled)
            {
                return new NoOpIdempotencyStore();
            }
            return new InMemoryIdempotencyStore(
                sp.GetRequiredService<IClock>(),
                sp.GetRequiredService<IOptions<IdempotencyOptions>>());
        });

        return services;
    }

    private sealed class NoOpIdempotencyStore : IIdempotencyStore
    {
        public Task<IdempotencyRecord?> TryGetAsync(string key, CancellationToken cancellationToken = default) =>
            Task.FromResult<IdempotencyRecord?>(null);

        public Task SaveAsync(IdempotencyRecord record, CancellationToken cancellationToken = default) =>
            Task.CompletedTask;

        public Task<int> EvictExpiredAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult(0);
    }
}
