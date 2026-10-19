using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Notifications.DependencyInjection;

namespace Platform.Notifications.Testing;

/// <summary>Development/test registration helpers.</summary>
public static class DependencyInjection
{
    /// <summary>Registers an in-memory provider explicitly for development or tests.</summary>
    public static IServiceCollection AddPlatformNotificationsDevelopment(this IServiceCollection services)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.AddPlatformNotifications();
        services.TryAddSingleton<InMemoryNotificationProvider>();
        services.TryAddSingleton<Platform.Mailing.IMailService>(sp => sp.GetRequiredService<InMemoryNotificationProvider>());
        services.TryAddSingleton<Platform.Notifications.ISmsSender>(sp => sp.GetRequiredService<InMemoryNotificationProvider>());
        services.AddSingleton<Platform.Notifications.INotificationProviderStatusSource>(sp => sp.GetRequiredService<InMemoryNotificationProvider>());
        return services;
    }
}
