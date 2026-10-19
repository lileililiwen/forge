using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Hosting;
using Platform.Core.Time;
using Platform.Eventing.Contracts;

namespace Platform.Eventing.EfCore.DependencyInjection;

/// <summary>Registration helpers for optional EF Core durable eventing.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers stores for an application-owned EF Core context.</summary>
    public static IServiceCollection AddPlatformEventingEfCore<TDbContext>(
        this IServiceCollection services,
        Action<DurableEventingOptions>? configure = null)
        where TDbContext : DbContext
    {
        ArgumentNullException.ThrowIfNull(services);
        services.AddOptions<DurableEventingOptions>().Configure(configure ?? (_ => { })).Validate(options => Validate(options), "Durable eventing options are invalid.");
        services.TryAddSingleton<IClock, SystemClock>();
        services.TryAddScoped<IOutboxStore>(sp => new EfCoreOutboxStore(
            sp.GetRequiredService<TDbContext>(),
            sp.GetRequiredService<IClock>(),
            sp.GetRequiredService<IOptions<DurableEventingOptions>>().Value));
        services.TryAddScoped<IInboxStore>(sp => new EfCoreInboxStore(
            sp.GetRequiredService<TDbContext>(),
            sp.GetRequiredService<IClock>(),
            sp.GetRequiredService<IOptions<DurableEventingOptions>>().Value));
        return services;
    }

    /// <summary>Registers the hosted outbox dispatcher using application-supplied stores and publisher.</summary>
    public static IServiceCollection AddPlatformDurableOutboxDispatcher(
        this IServiceCollection services,
        string workerId)
    {
        ArgumentNullException.ThrowIfNull(services);
        if (string.IsNullOrWhiteSpace(workerId)) throw new ArgumentException("Worker id is required.", nameof(workerId));
        services.AddSingleton<DurableOutboxDispatcher>(sp => new DurableOutboxDispatcher(
            sp.GetRequiredService<IOutboxStore>(),
            sp.GetRequiredService<IDurableEventPublisher>(),
            sp.GetRequiredService<IClock>(),
            sp.GetRequiredService<IOptions<DurableEventingOptions>>().Value,
            sp.GetRequiredService<ILogger<DurableOutboxDispatcher>>(),
            workerId));
        services.AddSingleton<IHostedService>(sp => sp.GetRequiredService<DurableOutboxDispatcher>());
        return services;
    }

    private static bool Validate(DurableEventingOptions options)
    {
        options.Validate();
        return true;
    }
}
