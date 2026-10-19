using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Core.Time;
using Platform.Webhooks.Contracts.Common;
using Platform.Webhooks.Contracts.Inbound;
using Platform.Webhooks.Contracts.Outbound;
using Platform.Webhooks.Contracts.Security;

namespace Platform.Webhooks.Contracts.DependencyInjection;

/// <summary>Registration helpers for webhook contracts and in-memory stores.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the in-memory webhook stores, default SSRF validator, default backend status, and clock.</summary>
    public static IServiceCollection AddPlatformWebhooks(this IServiceCollection services, Action<WebhookOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);
        var options = new WebhookOptions();
        configure?.Invoke(options);
        options.Validate();
        services.AddOptions<WebhookOptions>().Configure(_ => { });
        services.AddSingleton(options);
        services.TryAddSingleton<IClock, SystemClock>();
        services.TryAddSingleton<IWebhookInboxStore>(_ => new InMemoryWebhookInboxStore(options));
        services.TryAddSingleton<IWebhookSubscriptionStore>(_ => new InMemoryWebhookSubscriptionStore());
        services.TryAddSingleton<IWebhookDeliveryStore>(_ => new InMemoryWebhookDeliveryStore());
        services.TryAddSingleton<ISsrfTargetValidator>(_ => new SsrfTargetValidator(options));
        services.TryAddSingleton<IWebhookBackendStatusProvider>(_ => new InMemoryWebhookBackendStatusProvider(options));
        return services;
    }
}
