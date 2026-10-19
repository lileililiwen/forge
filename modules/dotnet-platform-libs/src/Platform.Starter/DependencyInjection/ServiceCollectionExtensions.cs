using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Platform.Admin.AspNetCore;
using Platform.Identity.AspNetCore;
using Platform.Mailing.DependencyInjection;
using Platform.Web.DependencyInjection;

namespace Platform.Starter.DependencyInjection;

/// <summary>Composes independently adoptable platform capabilities.</summary>
public static class ServiceCollectionExtensions
{
    /// <summary>Registers the web runtime and no optional providers.</summary>
    public static IServiceCollection AddPlatformApplication(this IServiceCollection services) => services.AddPlatformApplication(_ => { });

    /// <summary>Registers explicitly enabled platform capabilities.</summary>
    public static IServiceCollection AddPlatformApplication(this IServiceCollection services, Action<PlatformApplicationOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);
        var options = new PlatformApplicationOptions();
        configure(options);
        services.AddOptions<PlatformApplicationOptions>()
            .Configure(configure)
            .Validate(value => value.Validate().Count == 0, "Platform application options are invalid.")
            .ValidateOnStart();
        if (options.EnableWeb) services.AddPlatformWeb();
        if (options.EnableIdentity) services.AddPlatformIdentity();
        if (options.EnableAdmin) services.AddPlatformAdmin();
        if (options.EnableNotifications || options.EnableSms) services.AddPlatformMailing();
        services.TryAddSingleton<IPlatformApplicationStatus>(sp => new PlatformApplicationStatus(sp.GetRequiredService<IOptions<PlatformApplicationOptions>>().Value));
        services.AddSingleton<Platform.Web.IPlatformConfigurationValidator, PlatformApplicationConfigurationValidator>();
        return services;
    }

    /// <summary>Adds the starter middleware in the documented order.</summary>
    public static IApplicationBuilder UsePlatformApplication(this IApplicationBuilder app)
    {
        ArgumentNullException.ThrowIfNull(app);
        var options = app.ApplicationServices.GetRequiredService<IOptions<PlatformApplicationOptions>>().Value;
        if (options.EnableWeb) app.UsePlatformWeb();
        return app;
    }

    /// <summary>Maps the shared runtime endpoints when the web capability is enabled.</summary>
    public static IEndpointRouteBuilder MapPlatformApplicationEndpoints(this IEndpointRouteBuilder endpoints)
    {
        ArgumentNullException.ThrowIfNull(endpoints);
        var options = endpoints.ServiceProvider.GetRequiredService<IOptions<PlatformApplicationOptions>>().Value;
        if (options.EnableWeb) endpoints.MapPlatformRuntimeEndpoints();
        if (options.EnableAdmin) endpoints.MapPlatformAdminEndpoints();
        return endpoints;
    }
}
