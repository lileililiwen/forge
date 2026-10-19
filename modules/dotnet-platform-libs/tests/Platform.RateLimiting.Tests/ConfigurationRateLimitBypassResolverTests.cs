using Microsoft.Extensions.Options;

namespace Platform.RateLimiting.Tests;

public class ConfigurationRateLimitBypassResolverTests
{
    [Fact]
    public void Empty_token_is_not_bypassed()
    {
        var resolver = NewResolver(Array.Empty<string>());
        var context = new HttpContextAbstraction(BypassToken: null);

        var decision = resolver.Evaluate(context);

        Assert.False(decision.Allowed);
    }

    [Fact]
    public void Whitespace_token_is_not_bypassed()
    {
        var resolver = NewResolver(new[] { "token-1" });
        var context = new HttpContextAbstraction(BypassToken: "   ");

        var decision = resolver.Evaluate(context);

        Assert.False(decision.Allowed);
    }

    [Fact]
    public void Matching_token_bypasses()
    {
        var resolver = NewResolver(new[] { "token-1", "token-2" });
        var context = new HttpContextAbstraction(BypassToken: "token-2");

        var decision = resolver.Evaluate(context);

        Assert.True(decision.Allowed);
        Assert.Equal("token-2", decision.Label);
    }

    [Fact]
    public void Token_match_is_case_insensitive()
    {
        var resolver = NewResolver(new[] { "Service-Token" });
        var context = new HttpContextAbstraction(BypassToken: "service-token");

        var decision = resolver.Evaluate(context);

        Assert.True(decision.Allowed);
    }

    [Fact]
    public void Non_matching_token_is_not_bypassed()
    {
        var resolver = NewResolver(new[] { "token-1" });
        var context = new HttpContextAbstraction(BypassToken: "token-2");

        var decision = resolver.Evaluate(context);

        Assert.False(decision.Allowed);
        Assert.Null(decision.Label);
    }

    [Fact]
    public void Evaluate_rejects_null_context()
    {
        var resolver = NewResolver(Array.Empty<string>());

        Assert.Throws<ArgumentNullException>(() => resolver.Evaluate(null!));
    }

    [Fact]
    public void Ctor_rejects_null_options()
    {
        Assert.Throws<ArgumentNullException>(
            () => new ConfigurationRateLimitBypassResolver(null!));
    }

    private static ConfigurationRateLimitBypassResolver NewResolver(IReadOnlyList<string> tokens)
    {
        var options = Microsoft.Extensions.Options.Options.Create(new RateLimitingOptions
        {
            BypassTokens = tokens,
        });
        return new ConfigurationRateLimitBypassResolver(options);
    }
}
