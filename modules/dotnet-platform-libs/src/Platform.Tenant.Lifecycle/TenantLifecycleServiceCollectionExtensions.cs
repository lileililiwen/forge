using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Platform.Tenant.Lifecycle.Contracts;

namespace Platform.Tenant.Lifecycle;

/// <summary>DI helpers for the tenant lifecycle orchestrator.</summary>
public static class TenantLifecycleServiceCollectionExtensions
{
    /// <summary>Registers the in-memory store, scope callback, and orchestrator for development and test purposes.</summary>
    public static IServiceCollection AddPlatformTenantLifecycle(this IServiceCollection services)
        => services.AddPlatformTenantLifecycle(_ => { });

    /// <summary>Registers the in-memory store, scope callback, and orchestrator. Production callers should replace the store with a durable adapter.</summary>
    public static IServiceCollection AddPlatformTenantLifecycle(this IServiceCollection services, Action<TenantLifecycleOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        var options = new TenantLifecycleOptions();
        configure(options);
        services.AddOptions<TenantLifecycleOptions>().Configure(configure);
        services.TryAddSingleton<ITenantLifecycleStore, InMemoryTenantLifecycleStore>();
        services.TryAddSingleton<ITenantLifecycleScopeCallback, NoOpTenantLifecycleScopeCallback>();
        services.TryAddSingleton<TenantLifecycleWorkflowRegistry>();
        services.TryAddSingleton<ITenantLifecycleWorkflowRegistry>(sp => sp.GetRequiredService<TenantLifecycleWorkflowRegistry>());
        services.TryAddSingleton<TenantLifecycleOrchestrator>(sp =>
        {
            var orchestrator = new TenantLifecycleOrchestrator(
                sp.GetRequiredService<ITenantLifecycleStore>(),
                sp.GetRequiredService<ITenantLifecycleScopeCallback>(),
                sp.GetService<ILogger<TenantLifecycleOrchestrator>>());
            orchestrator.AttachRegistry(sp.GetRequiredService<ITenantLifecycleWorkflowRegistry>());
            return orchestrator;
        });
        services.TryAddSingleton<ITenantLifecycleOrchestrator>(sp => sp.GetRequiredService<TenantLifecycleOrchestrator>());
        return services;
    }
}

/// <summary>Configuration placeholder for <see cref="TenantLifecycleServiceCollectionExtensions.AddPlatformTenantLifecycle(IServiceCollection, Action{TenantLifecycleOptions})"/>.</summary>
public sealed class TenantLifecycleOptions
{
}

/// <summary>No-op scope callback used when an application does not register a real multitenancy adapter. Tenant-scoped steps run without an installed scope.</summary>
public sealed class NoOpTenantLifecycleScopeCallback : ITenantLifecycleScopeCallback
{
    /// <inheritdoc />
    public IDisposable BeginTenantScope(string tenantId) => NoOpDisposable.Instance;

    private sealed class NoOpDisposable : IDisposable
    {
        public static readonly NoOpDisposable Instance = new();
        public void Dispose() { }
    }
}
