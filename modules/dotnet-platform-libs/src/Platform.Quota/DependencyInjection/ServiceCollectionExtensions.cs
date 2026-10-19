using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Core.Time;
using Platform.Quota.Contracts;
using Platform.Quota.Stores;

namespace Platform.Quota.DependencyInjection;

/// <summary>Registration helpers for quota contracts and the local store.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the thread-safe in-memory quota store.</summary>
    public static IServiceCollection AddPlatformQuota(this IServiceCollection services, Action<QuotaOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        var options = new QuotaOptions();
        configure?.Invoke(options);
        options.Validate();
        services.TryAddSingleton<IClock, SystemClock>();
        services.AddSingleton(options);
        services.TryAddSingleton<IQuotaStore, InMemoryQuotaStore>();
        return services;
    }
}
