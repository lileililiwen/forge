using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Privacy.Contracts;
using Platform.Privacy.Stores;

namespace Platform.Privacy.DependencyInjection;

/// <summary>Registration helpers for privacy contracts and the local store.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the registry and the thread-safe in-memory consent store.</summary>
    public static IServiceCollection AddPlatformPrivacy(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddSingleton<SubjectRightsRegistry>();
        services.TryAddSingleton<IConsentStore, InMemoryConsentStore>();
        return services;
    }
}
