using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Platform.Web.Cors;
using Platform.Web.Cors.DependencyInjection;

namespace Platform.Web.Edge.Tests;

public sealed class CorsOptionsTests
{
    [Fact]
    public void Wildcard_origin_with_credentials_is_rejected()
    {
        var options = new PlatformWebCorsOptions
        {
            Environment = "Production",
            Policies =
            {
                new PlatformWebCorsPolicyOptions
                {
                    Name = "default",
                    AllowedOrigins = { "*" },
                    AllowCredentials = true,
                }
            }
        };

        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("Wildcard origins cannot be combined with credentials"));
        Assert.Contains(errors, e => e.Contains("Wildcard origins are not allowed in production"));
    }

    [Fact]
    public void Production_requires_at_least_one_origin()
    {
        var options = new PlatformWebCorsOptions
        {
            Environment = "Production",
            Policies =
            {
                new PlatformWebCorsPolicyOptions { Name = "default", AllowedOrigins = { } }
            }
        };
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("At least one allowed origin is required in production"));
    }

    [Fact]
    public void Non_https_origin_is_allowed_in_development_but_not_in_production()
    {
        var policy = new PlatformWebCorsPolicyOptions
        {
            Name = "default",
            AllowedOrigins = { "http://example.com" }
        };
        var options = new PlatformWebCorsOptions
        {
            Environment = "Development",
            Policies = { policy }
        };
        Assert.Empty(options.Validate());

        options.Environment = "Production";
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("must use HTTPS in production"));
    }

    [Fact]
    public void Localhost_is_always_allowed()
    {
        var options = new PlatformWebCorsOptions
        {
            Environment = "Production",
            Policies =
            {
                new PlatformWebCorsPolicyOptions
                {
                    Name = "default",
                    AllowedOrigins = { "http://localhost" }
                }
            }
        };
        Assert.Empty(options.Validate());
    }

    [Fact]
    public void Duplicate_policy_names_are_rejected()
    {
        var options = new PlatformWebCorsOptions
        {
            Policies =
            {
                new PlatformWebCorsPolicyOptions { Name = "default" },
                new PlatformWebCorsPolicyOptions { Name = "default" }
            }
        };
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("Duplicate CORS policy name"));
    }

    [Fact]
    public void Empty_policy_collection_is_rejected()
    {
        var options = new PlatformWebCorsOptions { Environment = "Development" };
        var errors = options.Validate();
        Assert.Contains(errors, e => e.Contains("At least one CORS policy must be configured"));
    }

    [Fact]
    public void Options_validation_is_invoked_at_resolution_time()
    {
        var services = new ServiceCollection();
        services.AddPlatformWebCors(options =>
        {
            options.Environment = "Production";
            options.Policies.Add(new PlatformWebCorsPolicyOptions
            {
                Name = "default",
                AllowedOrigins = { "*" },
                AllowCredentials = true,
            });
        });
        using var sp = services.BuildServiceProvider();
        Assert.Throws<OptionsValidationException>(() => sp.GetRequiredService<IOptions<PlatformWebCorsOptions>>().Value);
    }
}
