using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Persistence.EfCore.Audit;
using Platform.Persistence.EfCore.Interceptors;

namespace Platform.Persistence.EfCore.DependencyInjection;

/// <summary>Registration helpers for optional EF Core conventions.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers persistence options and replaceable convention services.</summary>
    public static IServiceCollection AddPlatformPersistenceEfCore(this IServiceCollection services) =>
        services.AddPlatformPersistenceEfCore(_ => { });

    /// <summary>Registers persistence options and configures their opt-in switches.</summary>
    public static IServiceCollection AddPlatformPersistenceEfCore(this IServiceCollection services, Action<PersistenceOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<PersistenceOptions>().Configure(configure);
        services.TryAddSingleton<IClock, SystemClock>();
        services.TryAddSingleton<IActorAccessor, AnonymousActorAccessor>();
        services.TryAddSingleton<PlatformSaveChangesInterceptor>();
        return services;
    }

    private sealed class AnonymousActorAccessor : IActorAccessor
    {
        public string? SubjectId => null;
    }
}
