using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;

namespace Platform.Identity.AspNetCore;

/// <summary>JWT configuration registration for the platform identity host adapter.</summary>
public static class JwtAuthenticationServiceCollectionExtensions
{
    /// <summary>Registers JWT authentication options validated on startup. Disabled until explicitly enabled.</summary>
    public static IServiceCollection AddPlatformIdentityJwt(this IServiceCollection services) => services.AddPlatformIdentityJwt(_ => { });

    /// <summary>Registers JWT authentication options validated on startup with explicit configuration.</summary>
    public static IServiceCollection AddPlatformIdentityJwt(this IServiceCollection services, Action<PlatformIdentityJwtOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        services.AddOptions<PlatformIdentityJwtOptions>().Configure(configure).ValidateOnStart();
        services.TryAddEnumerable(ServiceDescriptor.Singleton<IValidateOptions<PlatformIdentityJwtOptions>, PlatformIdentityJwtOptionsValidator>());
        return services;
    }
}
