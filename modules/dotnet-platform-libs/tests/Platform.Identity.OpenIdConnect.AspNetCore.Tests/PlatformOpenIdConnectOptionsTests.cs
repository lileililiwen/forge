using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

public class PlatformOpenIdConnectOptionsTests
{
    [Fact]
    public void Missing_issuer_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions { ClientId = "client" });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("Issuer", StringComparison.Ordinal));
    }

    [Fact]
    public void Plain_http_issuer_is_rejected_outside_local_development()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "http://issuer.test",
            ClientId = "client",
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("HTTPS", StringComparison.Ordinal));
    }

    [Fact]
    public void Plain_http_issuer_is_rejected_for_a_remote_host_even_in_development()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "http://issuer.test",
            ClientId = "client",
            AllowInsecureHttpIssuer = true,
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("loopback", StringComparison.Ordinal));
    }

    [Fact]
    public void Plain_http_loopback_issuer_is_accepted_in_development()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "http://localhost:8080/realms/demo",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "openid" },
            AllowInsecureHttpIssuer = true,
        });

        Assert.True(result.Succeeded);
    }

    [Fact]
    public void Missing_client_id_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions { Issuer = "https://issuer.test" });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("ClientId", StringComparison.Ordinal));
    }

    [Fact]
    public void Missing_sign_in_scheme_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("SignInScheme", StringComparison.Ordinal));
    }

    [Fact]
    public void Relative_callback_path_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            CallbackPath = "signin-oidc",
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("CallbackPath", StringComparison.Ordinal));
    }

    [Fact]
    public void Scopes_must_include_openid()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "profile" },
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("openid", StringComparison.Ordinal));
    }

    [Fact]
    public void Empty_scope_allowlist_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("Scopes", StringComparison.Ordinal));
    }

    [Fact]
    public void Confidential_client_without_a_secret_reference_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "openid" },
            ClientAuthentication = PlatformOpenIdConnectClientAuthentication.ClientSecret,
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("ClientSecretReference", StringComparison.Ordinal));
    }

    [Fact]
    public void Confidential_client_with_an_unresolvable_secret_reference_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "openid" },
            ClientAuthentication = PlatformOpenIdConnectClientAuthentication.ClientSecret,
            ClientSecretReference = "env:PLATFORM_TEST_MISSING_SECRET",
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("could not be resolved", StringComparison.Ordinal));
    }

    [Fact]
    public void Excessive_clock_skew_is_rejected()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "openid" },
            ClockSkew = TimeSpan.FromMinutes(30),
        });

        Assert.True(result.Failed);
        Assert.Contains(result.Failures, failure => failure.Contains("ClockSkew", StringComparison.Ordinal));
    }

    [Fact]
    public void A_complete_configuration_is_accepted()
    {
        var result = Validate(new PlatformOpenIdConnectOptions
        {
            Issuer = "https://issuer.test",
            ClientId = "client",
            SignInScheme = "Cookies",
            Scopes = new[] { "openid", "profile" },
        });

        Assert.True(result.Succeeded);
    }

    [Fact]
    public void Startup_validation_fails_for_an_invalid_configuration()
    {
        var services = new ServiceCollection();
        services.AddPlatformOpenIdConnect(options =>
        {
            options.Issuer = string.Empty;
            options.ClientId = "client";
            options.SignInScheme = "Cookies";
            options.Scopes = new[] { "openid" };
        });

        using var provider = services.BuildServiceProvider();
        var options = provider.GetRequiredService<IOptions<PlatformOpenIdConnectOptions>>();

        Assert.Throws<OptionsValidationException>(() => options.Value);
    }

    private static ValidateOptionsResult Validate(PlatformOpenIdConnectOptions options)
    {
        var services = new ServiceCollection();
        services.AddSingleton<IPlatformOpenIdConnectSecretResolver, PlatformOpenIdConnectSecretResolver>();
        services.AddSingleton<IValidateOptions<PlatformOpenIdConnectOptions>, PlatformOpenIdConnectOptionsValidator>();
        using var provider = services.BuildServiceProvider();
        var validator = provider.GetRequiredService<IValidateOptions<PlatformOpenIdConnectOptions>>();
        return validator.Validate(null, options);
    }
}
