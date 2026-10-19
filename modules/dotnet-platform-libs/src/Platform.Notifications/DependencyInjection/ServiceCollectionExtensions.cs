using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Core.Time;
using Platform.Idempotency.DependencyInjection;
using Platform.Jobs.DependencyInjection;
using Platform.Mailing.DependencyInjection;

namespace Platform.Notifications.DependencyInjection;

/// <summary>Registers notification contracts and orchestration.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers notifications with no provider implementation.</summary>
    public static IServiceCollection AddPlatformNotifications(this IServiceCollection services) => services.AddPlatformNotifications(_ => { });

    /// <summary>Registers notifications and configures options.</summary>
    public static IServiceCollection AddPlatformNotifications(this IServiceCollection services, Action<NotificationOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<NotificationOptions>().Configure(configure).Validate(o => o.Validate().Count == 0, "Notification options are invalid.");
        services.AddPlatformMailing();
        services.AddPlatformJobs();
        services.AddPlatformIdempotency();
        services.TryAddSingleton<IClock>(_ => new SystemClock());
        services.TryAddSingleton<INotificationDispatcher, NotificationDispatcher>();
        services.TryAddSingleton<INotificationProviderStatusSource, EmptyNotificationProviderStatusSource>();
        return services;
    }

    private sealed class EmptyNotificationProviderStatusSource : INotificationProviderStatusSource
    {
        public IReadOnlyList<NotificationProviderStatus> GetStatuses() => Array.Empty<NotificationProviderStatus>();
    }
}
