using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Billing.Contracts.Providers;

namespace Platform.Billing.Stripe;

/// <summary>Registration helpers for Stripe.</summary>
public static class DependencyInjection
{
    /// <summary>Registers the Stripe provider as an opt-in named HTTP client.</summary>
    public static IServiceCollection AddPlatformStripe(this IServiceCollection services, Action<StripeOptions>? configure = null)
    {
        services.AddOptions<StripeOptions>(); if (configure is not null) services.Configure(configure);
        services.AddSingleton(sp => sp.GetRequiredService<IOptions<StripeOptions>>().Value);
        services.AddHttpClient<StripeBillingProvider>(); services.AddSingleton<IBillingProvider>(sp => sp.GetRequiredService<StripeBillingProvider>());
        return services;
    }
}
