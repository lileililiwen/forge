namespace Platform.RateLimiting.Tests;

public class RateLimitingOptionsTests
{
    [Fact]
    public void Defaults_apply_documented_values()
    {
        var options = new RateLimitingOptions();

        Assert.Equal("RateLimiting", RateLimitingOptions.SectionName);
        Assert.NotNull(options.Policies);
        Assert.Equal(5, options.Policies.Entries.Count);
        Assert.Empty(options.BypassTokens);
    }
}
