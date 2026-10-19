namespace Platform.RateLimiting.Tests;

public class RateLimitPoliciesTests
{
    [Fact]
    public void Default_catalog_contains_documented_policies()
    {
        var policies = RateLimitPolicies.Default();

        Assert.NotNull(policies.Find("feed"));
        Assert.NotNull(policies.Find("search"));
        Assert.NotNull(policies.Find("uploads"));
        Assert.NotNull(policies.Find("downloads"));
        Assert.NotNull(policies.Find("account-recovery"));
    }

    [Fact]
    public void Default_feed_policy_is_60_per_minute()
    {
        var policies = RateLimitPolicies.Default();
        var feed = policies.Find("feed");

        Assert.NotNull(feed);
        Assert.Equal(60, feed!.Limit);
        Assert.Equal(60, feed.WindowSeconds);
    }

    [Fact]
    public void Find_returns_null_for_unknown_policy()
    {
        var policies = RateLimitPolicies.Default();

        Assert.Null(policies.Find("unknown"));
        Assert.Null(policies.Find(""));
        Assert.Null(policies.Find("   "));
    }

    [Fact]
    public void Ctor_drops_invalid_entries()
    {
        var policies = new RateLimitPolicies(new[]
        {
            new RateLimitPolicyOptions { Name = "valid", Limit = 5, WindowSeconds = 60 },
            new RateLimitPolicyOptions { Name = "", Limit = 5, WindowSeconds = 60 },
            new RateLimitPolicyOptions { Name = "negative", Limit = -1, WindowSeconds = 60 },
            new RateLimitPolicyOptions { Name = "zero-window", Limit = 5, WindowSeconds = 0 },
            new RateLimitPolicyOptions { Name = "duplicate", Limit = 5, WindowSeconds = 60 },
            new RateLimitPolicyOptions { Name = "duplicate", Limit = 99, WindowSeconds = 60 },
        });

        Assert.Equal(2, policies.Entries.Count);
        Assert.Equal("valid", policies.Entries[0].Name);
        Assert.Equal("duplicate", policies.Entries[1].Name);
        Assert.Equal(5, policies.Entries[1].Limit);
    }

    [Fact]
    public void Ctor_rejects_null_entries()
    {
        Assert.Throws<ArgumentNullException>(() => new RateLimitPolicies(null!));
    }
}
