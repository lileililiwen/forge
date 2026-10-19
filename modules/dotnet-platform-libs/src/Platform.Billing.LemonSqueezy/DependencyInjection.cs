using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Billing.Contracts.Providers;

namespace Platform.Billing.LemonSqueezy;

/// <summary>Registration helpers for Lemon Squeezy.</summary>
public static class DependencyInjection
{
    /// <summary>Registers the Lemon Squeezy provider as an opt-in HTTP client.</summary>
    public static IServiceCollection AddPlatformLemonSqueezy(this IServiceCollection services, Action<LemonSqueezyOptions>? configure = null)
    { services.AddOptions<LemonSqueezyOptions>(); if (configure is not null) services.Configure(configure); services.AddSingleton(sp => sp.GetRequiredService<IOptions<LemonSqueezyOptions>>().Value); services.AddHttpClient<LemonSqueezyBillingProvider>(); services.AddSingleton<IBillingProvider>(sp => sp.GetRequiredService<LemonSqueezyBillingProvider>()); return services; }
}
