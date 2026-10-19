using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.EfCore.Inbound;
using Platform.Webhooks.EfCore.Outbound;

namespace Platform.Webhooks.EfCore.DependencyInjection;

/// <summary>Registration helpers for the optional EF Core webhook stores.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the EF Core inbox and delivery stores using the supplied <typeparamref name="TContext"/>.</summary>
    public static IServiceCollection AddPlatformWebhooksEfCore<TContext>(this IServiceCollection services) where TContext : DbContext
    {
        ArgumentNullException.ThrowIfNull(services);
        services.TryAddSingleton<IClock, SystemClock>();
        services.AddScoped<IWebhookInboxStore>(sp => new EfCoreWebhookInboxStore(sp.GetRequiredService<TContext>(), sp.GetRequiredService<IClock>()));
        services.AddScoped<IWebhookDeliveryStore>(sp => new EfCoreWebhookDeliveryStore(sp.GetRequiredService<TContext>()));
        return services;
    }
}
