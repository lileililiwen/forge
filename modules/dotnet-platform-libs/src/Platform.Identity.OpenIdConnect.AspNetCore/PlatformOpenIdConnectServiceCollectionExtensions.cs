using System.Security.Claims;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.OpenIdConnect;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Options;
using Microsoft.IdentityModel.Protocols.OpenIdConnect;
using Microsoft.IdentityModel.Tokens;

namespace Platform.Identity.OpenIdConnect.AspNetCore;

/// <summary>
/// Registers the opt-in ASP.NET Core OpenID Connect adapter. The adapter is never
/// registered implicitly: an application opts in with one explicit call.
/// </summary>
public static class PlatformOpenIdConnectServiceCollectionExtensions
{
    /// <summary>Registers the adapter with explicit options.</summary>
    public static IServiceCollection AddPlatformOpenIdConnect(
        this IServiceCollection services,
        Action<PlatformOpenIdConnectOptions> configure)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(configure);

        var options = new PlatformOpenIdConnectOptions();
        configure(options);
        return services.AddPlatformOpenIdConnect(options);
    }

    /// <summary>Registers the adapter with a prepared options instance.</summary>
    public static IServiceCollection AddPlatformOpenIdConnect(
        this IServiceCollection services,
        PlatformOpenIdConnectOptions options)
    {
        ArgumentNullException.ThrowIfNull(services);
        ArgumentNullException.ThrowIfNull(options);

        if (string.IsNullOrWhiteSpace(options.SchemeName))
        {
            throw new ArgumentException("The scheme name is required.", nameof(options));
        }

        services.TryAddSingleton<IPlatformOpenIdConnectSecretResolver, PlatformOpenIdConnectSecretResolver>();
        services.TryAddSingleton<IPlatformOpenIdConnectStateReplayGuard>(provider =>
            new InMemoryPlatformOpenIdConnectStateReplayGuard(
                TimeProvider.System,
                provider.GetRequiredService<IOptions<PlatformOpenIdConnectOptions>>().Value.StateLifetime));

        services.AddOptions<PlatformOpenIdConnectOptions>()
            .Configure(configured => Copy(options, configured))
            .ValidateOnStart();
        services.TryAddEnumerable(
            ServiceDescriptor.Singleton<IValidateOptions<PlatformOpenIdConnectOptions>, PlatformOpenIdConnectOptionsValidator>());

        services.AddAuthentication().AddOpenIdConnect(options.SchemeName, _ => { });
        services.TryAddEnumerable(
            ServiceDescriptor.Singleton<IConfigureOptions<OpenIdConnectOptions>, PlatformOpenIdConnectOptionsConfigurator>());
        services.TryAddEnumerable(
            ServiceDescriptor.Singleton<IPostConfigureOptions<OpenIdConnectOptions>, PlatformOpenIdConnectPostConfigureOptions>());

        return services;
    }

    private static void Copy(PlatformOpenIdConnectOptions source, PlatformOpenIdConnectOptions target)
    {
        target.Issuer = source.Issuer;
        target.ClientId = source.ClientId;
        target.CallbackPath = source.CallbackPath;
        target.SignInScheme = source.SignInScheme;
        target.Scopes = source.Scopes;
        target.ClientAuthentication = source.ClientAuthentication;
        target.ClientSecretReference = source.ClientSecretReference;
        target.AllowInsecureHttpIssuer = source.AllowInsecureHttpIssuer;
        target.RoleClaimType = source.RoleClaimType;
        target.ClockSkew = source.ClockSkew;
        target.StateLifetime = source.StateLifetime;
        target.IdentityResolution = source.IdentityResolution;
        target.SchemeName = source.SchemeName;
        target.BackchannelHttpHandler = source.BackchannelHttpHandler;
        target.ConfigureEvents = source.ConfigureEvents;
    }
}

/// <summary>Maps the adapter's options onto the ASP.NET Core OpenID Connect options.</summary>
internal sealed class PlatformOpenIdConnectOptionsConfigurator : IConfigureNamedOptions<OpenIdConnectOptions>
{
    private readonly IOptions<PlatformOpenIdConnectOptions> _platformOptions;
    private readonly IPlatformOpenIdConnectSecretResolver _secretResolver;

    public PlatformOpenIdConnectOptionsConfigurator(
        IOptions<PlatformOpenIdConnectOptions> platformOptions,
        IPlatformOpenIdConnectSecretResolver secretResolver)
    {
        _platformOptions = platformOptions;
        _secretResolver = secretResolver;
    }

    public void Configure(OpenIdConnectOptions options) => Configure(Options.DefaultName, options);

    public void Configure(string? name, OpenIdConnectOptions options)
    {
        var platform = _platformOptions.Value;
        if (!string.Equals(name, platform.SchemeName, StringComparison.Ordinal))
        {
            return;
        }

        options.Authority = platform.Issuer;
        options.ClientId = platform.ClientId;
        options.CallbackPath = new PathString(platform.CallbackPath);
        options.SignInScheme = platform.SignInScheme;
        options.ResponseType = OpenIdConnectResponseType.Code;
        options.UsePkce = true;
        options.SaveTokens = false;
        options.RequireHttpsMetadata = !platform.AllowInsecureHttpIssuer;
        options.GetClaimsFromUserInfoEndpoint = false;
        options.MapInboundClaims = false;
        var backchannel = new PlatformOpenIdConnectBackchannelHandler(
            platform.BackchannelHttpHandler ?? new HttpClientHandler { UseCookies = false });
        options.BackchannelHttpHandler = backchannel;
        options.Backchannel = new HttpClient(backchannel) { Timeout = options.BackchannelTimeout };

        options.Scope.Clear();
        foreach (var scope in platform.Scopes)
        {
            options.Scope.Add(scope);
        }

        if (platform.ClientAuthentication == PlatformOpenIdConnectClientAuthentication.ClientSecret)
        {
            var secret = _secretResolver.Resolve(platform.ClientSecretReference!);
            if (string.IsNullOrWhiteSpace(secret))
            {
                throw new PlatformOpenIdConnectException(
                    PlatformOpenIdConnectFailureKind.InvalidConfiguration,
                    "client_secret_unresolved",
                    "The OpenID Connect configuration is not valid.");
            }

            options.ClientSecret = secret;
        }

        var events = new OpenIdConnectEvents();
        platform.ConfigureEvents?.Invoke(events);
        new PlatformOpenIdConnectEventBinder(platform).Bind(events);
        options.Events = events;
    }
}

/// <summary>Applies the adapter's final token validation posture.</summary>
internal sealed class PlatformOpenIdConnectPostConfigureOptions : IPostConfigureOptions<OpenIdConnectOptions>
{
    private readonly IOptions<PlatformOpenIdConnectOptions> _platformOptions;

    public PlatformOpenIdConnectPostConfigureOptions(IOptions<PlatformOpenIdConnectOptions> platformOptions)
    {
        _platformOptions = platformOptions;
    }

    public void PostConfigure(string? name, OpenIdConnectOptions options)
    {
        var platform = _platformOptions.Value;
        if (!string.Equals(name, platform.SchemeName, StringComparison.Ordinal))
        {
            return;
        }

        options.TokenValidationParameters ??= new TokenValidationParameters();
        options.TokenValidationParameters.ValidIssuer = platform.Issuer.TrimEnd('/');
        options.TokenValidationParameters.ValidAudience = platform.ClientId;
        options.TokenValidationParameters.ValidateIssuer = true;
        options.TokenValidationParameters.ValidateAudience = true;
        options.TokenValidationParameters.ValidateLifetime = true;
        options.TokenValidationParameters.RequireExpirationTime = true;
        options.TokenValidationParameters.ClockSkew = platform.ClockSkew;
        options.TokenValidationParameters.NameClaimType = "name";
        if (!string.IsNullOrWhiteSpace(platform.RoleClaimType))
        {
            options.TokenValidationParameters.RoleClaimType = platform.RoleClaimType;
        }
    }
}
