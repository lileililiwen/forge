using System.Net;
using System.Security.Claims;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.Cookies;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

/// <summary>Builds the application under test and drives the sign-in journey.</summary>
internal sealed class OidcTestApp : IAsyncDisposable
{
    public const string CookieScheme = "TestCookies";

    private readonly WebApplication _app;
    private readonly HttpClient _client;

    private OidcTestApp(WebApplication app, HttpClient client)
    {
        _app = app;
        _client = client;
    }

    public HttpClient Client => _client;

    public static async Task<OidcTestApp> StartAsync(
        LocalOidcTestIssuer issuer,
        Action<PlatformOpenIdConnectOptions>? configure = null,
        Action<IServiceCollection>? configureServices = null,
        bool useFailoverHandler = false)
    {
        var builder = WebApplication.CreateBuilder(new WebApplicationOptions { EnvironmentName = "Testing" });
        builder.WebHost.UseTestServer();
        builder.Logging.ClearProviders();

        builder.Services
            .AddAuthentication(options =>
            {
                options.DefaultScheme = CookieScheme;
                options.DefaultChallengeScheme = PlatformOpenIdConnectOptions.DefaultSchemeName;
            })
            .AddCookie(CookieScheme);

        builder.Services.AddPlatformOpenIdConnect(options =>
        {
            options.Issuer = issuer.Issuer;
            options.ClientId = issuer.ClientId;
            options.SignInScheme = CookieScheme;
            options.CallbackPath = PlatformOpenIdConnectOptions.DefaultCallbackPath;
            options.Scopes = new[] { "openid", "profile" };
            options.BackchannelHttpHandler = useFailoverHandler
                ? new FailoverTokenEndpointHandler(issuer.CreateHandler())
                : issuer.CreateHandler();
            configure?.Invoke(options);
        });

        configureServices?.Invoke(builder.Services);
        builder.Services.AddAuthorization();

        var app = builder.Build();
        app.Use(async (context, next) =>
        {
            try
            {
                await next();
            }
            catch (PlatformOpenIdConnectException failure)
            {
                context.Response.StatusCode = StatusCodes.Status400BadRequest;
                await context.Response.WriteAsync($"oidc-failure:{failure.Kind}:{failure.Reason}");
            }
        });
        app.UseAuthentication();
        app.UseAuthorization();
        app.MapGet("/protected", () => Results.Ok("protected")).RequireAuthorization();
        app.MapGet(
            "/whoami",
            async (HttpContext context) =>
            {
                var result = await context.AuthenticateAsync(OidcTestApp.CookieScheme);
                var user = result.Principal;
                PlatformOpenIdConnectPrincipal.TryGetExternalIdentity(
                    user!,
                    result.Properties,
                    out var identity);
                return Results.Json(new
                {
                    name = user?.Identity?.Name,
                    subject = user?.FindFirst("sub")?.Value,
                    issuer = identity?.Issuer,
                    externalSubject = identity?.Subject,
                    roles = user?.FindAll(ClaimTypes.Role).Select(claim => claim.Value).ToArray() ?? Array.Empty<string>(),
                });
            }).RequireAuthorization();

        await app.StartAsync();

        var handler = new CookieHandler(app.GetTestServer().CreateHandler());
        var client = new HttpClient(handler) { BaseAddress = new Uri("http://localhost") };

        return new OidcTestApp(app, client);
    }

    /// <summary>Requests a protected resource and returns the challenge response.</summary>
    public Task<HttpResponseMessage> ChallengeAsync() => _client.GetAsync("/protected");

    /// <summary>Requests a protected resource and returns its body.</summary>
    public Task<string> WhoAmIAsync() => _client.GetStringAsync("/whoami");

    public async ValueTask DisposeAsync()
    {
        _client.Dispose();
        await _app.StopAsync();
        await _app.DisposeAsync();
    }
}

/// <summary>Carries the cookies of one sign-in journey across requests.</summary>
internal sealed class CookieHandler : DelegatingHandler
{
    private readonly Dictionary<string, string> _cookies = new(StringComparer.OrdinalIgnoreCase);

    public CookieHandler(HttpMessageHandler innerHandler)
        : base(innerHandler)
    {
    }

    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken)
    {
        if (_cookies.Count > 0)
        {
            var header = string.Join("; ", _cookies.Select(cookie => $"{cookie.Key}={cookie.Value}"));
            request.Headers.Remove("Cookie");
            request.Headers.Add("Cookie", header);
        }

        var response = await base.SendAsync(request, cancellationToken);
        if (response.Headers.TryGetValues("Set-Cookie", out var values))
        {
            foreach (var value in values)
            {
                var pair = value.Split(';')[0];
                var separator = pair.IndexOf('=');
                if (separator > 0)
                {
                    _cookies[pair[..separator]] = pair[(separator + 1)..];
                }
            }
        }

        return response;
    }
}

/// <summary>Fails the token endpoint the way an unreachable provider does.</summary>
internal sealed class FailoverTokenEndpointHandler : DelegatingHandler
{
    public FailoverTokenEndpointHandler(HttpMessageHandler innerHandler)
        : base(innerHandler)
    {
    }

    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken)
    {
        if (request.RequestUri is not null
            && request.RequestUri.AbsolutePath.Equals("/token", StringComparison.Ordinal))
        {
            throw new HttpRequestException("The provider is unreachable.");
        }

        return base.SendAsync(request, cancellationToken);
    }
}
