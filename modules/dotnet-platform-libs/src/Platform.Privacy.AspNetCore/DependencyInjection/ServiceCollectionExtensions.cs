using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Platform.Privacy.AspNetCore.Cookies;
using Platform.Privacy.AspNetCore.Enforcement;
using Platform.Privacy.AspNetCore.State;
using Platform.Privacy.DependencyInjection;

namespace Platform.Privacy.AspNetCore.DependencyInjection;

/// <summary>Registration helpers for the privacy HTTP adapter.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers consent state, cookie service, and core privacy contracts.</summary>
    public static IServiceCollection AddPlatformPrivacyAspNetCore(
        this IServiceCollection services,
        Action<ConsentCookieOptions>? configure = null,
        UnresolvedSubjectBehavior unresolvedBehavior = UnresolvedSubjectBehavior.Deny)
    {
        ArgumentNullException.ThrowIfNull(services);
        services.AddPlatformPrivacy();
        var options = new ConsentCookieOptions();
        configure?.Invoke(options);
        options.Validate();
        services.AddSingleton(options);
        services.AddSingleton(typeof(UnresolvedSubjectBehavior), unresolvedBehavior);
        services.TryAddScoped<ConsentState>();
        services.TryAddSingleton<ConsentCookieService>();
        return services;
    }
}
