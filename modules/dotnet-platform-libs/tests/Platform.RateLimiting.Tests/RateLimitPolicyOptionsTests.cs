namespace Platform.RateLimiting.Tests;

public class RateLimitPolicyOptionsTests
{
    [Fact]
    public void Defaults_apply_safe_values()
    {
        var options = new RateLimitPolicyOptions();

        Assert.Equal(string.Empty, options.Name);
        Assert.Equal(0, options.Limit);
        Assert.Equal(0, options.WindowSeconds);
    }
}
