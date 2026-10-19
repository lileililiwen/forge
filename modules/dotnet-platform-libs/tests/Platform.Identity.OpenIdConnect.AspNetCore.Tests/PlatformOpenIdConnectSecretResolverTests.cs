using Microsoft.Extensions.Configuration;

namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

public class PlatformOpenIdConnectSecretResolverTests
{
    [Fact]
    public void A_configuration_reference_resolves_to_its_value()
    {
        var configuration = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["Identity:Oidc:ClientSecret"] = "configured-secret",
            })
            .Build();
        var resolver = new PlatformOpenIdConnectSecretResolver(configuration);

        var value = resolver.Resolve("config:Identity:Oidc:ClientSecret");

        Assert.Equal("configured-secret", value);
    }

    [Fact]
    public void An_unknown_configuration_reference_resolves_to_null()
    {
        var resolver = new PlatformOpenIdConnectSecretResolver(new ConfigurationBuilder().Build());

        Assert.Null(resolver.Resolve("config:Identity:Missing"));
    }

    [Fact]
    public void An_environment_reference_resolves_to_its_value()
    {
        const string name = "PLATFORM_OIDC_TEST_SECRET";
        Environment.SetEnvironmentVariable(name, "environment-secret");
        try
        {
            var resolver = new PlatformOpenIdConnectSecretResolver();

            Assert.Equal("environment-secret", resolver.Resolve($"env:{name}"));
        }
        finally
        {
            Environment.SetEnvironmentVariable(name, null);
        }
    }

    [Fact]
    public void An_unknown_reference_format_resolves_to_null()
    {
        var resolver = new PlatformOpenIdConnectSecretResolver();

        Assert.Null(resolver.Resolve("plain-secret-value"));
        Assert.Null(resolver.Resolve(string.Empty));
        Assert.Null(resolver.Resolve("config:"));
        Assert.Null(resolver.Resolve("env:"));
    }
}
