using Microsoft.Extensions.Options;
using Platform.Core.Time;

namespace Platform.RateLimiting.Tests;

public class InMemoryRateLimiterTests
{
    [Fact]
    public async Task First_request_within_window_is_allowed_with_remaining()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, new RateLimitingOptions
        {
            Policies = new RateLimitPolicies(new[]
            {
                new RateLimitPolicyOptions { Name = "p", Limit = 5, WindowSeconds = 60 },
            }),
        });

        var decision = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));

        Assert.True(decision.Allowed);
        Assert.Equal(5, decision.Limit);
        Assert.Equal(4, decision.Remaining);
        Assert.Equal(0, decision.RetryAfterSeconds);
    }

    [Fact]
    public async Task Burst_over_the_limit_is_denied_with_retry_after()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, NewOptions(limit: 5, windowSeconds: 60));

        for (var i = 0; i < 5; i++)
        {
            await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));
        }

        var denied = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));

        Assert.False(denied.Allowed);
        Assert.Equal(0, denied.Remaining);
        Assert.InRange(denied.RetryAfterSeconds, 1, 60);
    }

    [Fact]
    public async Task Window_rolls_over_after_window_seconds()
    {
        var clock = new MutableClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, NewOptions(limit: 1, windowSeconds: 60));

        var first = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));
        Assert.True(first.Allowed);

        clock.Advance(TimeSpan.FromSeconds(30));
        var blocked = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));
        Assert.False(blocked.Allowed);

        clock.Advance(TimeSpan.FromSeconds(31));
        var afterRollover = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));
        Assert.True(afterRollover.Allowed);
    }

    [Fact]
    public async Task Different_subjects_have_independent_buckets()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, NewOptions(limit: 1, windowSeconds: 60));

        var firstSubjectFirst = await limiter.CheckAsync(new RateLimitKey("p", "subject-1"));
        var secondSubjectFirst = await limiter.CheckAsync(new RateLimitKey("p", "subject-2"));

        Assert.True(firstSubjectFirst.Allowed);
        Assert.True(secondSubjectFirst.Allowed);
    }

    [Fact]
    public async Task Unknown_policy_throws()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, new RateLimitingOptions());

        await Assert.ThrowsAsync<ArgumentException>(
            async () => await limiter.CheckAsync(new RateLimitKey("unknown", "subject")));
    }

    [Fact]
    public async Task Empty_policy_throws()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, new RateLimitingOptions());

        await Assert.ThrowsAsync<ArgumentException>(
            async () => await limiter.CheckAsync(new RateLimitKey("", "subject")));
    }

    [Fact]
    public async Task Empty_subject_throws()
    {
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));
        var limiter = NewLimiter(clock, new RateLimitingOptions());

        await Assert.ThrowsAsync<ArgumentException>(
            async () => await limiter.CheckAsync(new RateLimitKey("p", "")));
    }

    [Fact]
    public void Ctor_rejects_null_dependencies()
    {
        var options = Microsoft.Extensions.Options.Options.Create(new RateLimitingOptions());
        var clock = new FixedClock(new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero));

        Assert.Throws<ArgumentNullException>(() => new InMemoryRateLimiter(null!, options));
        Assert.Throws<ArgumentNullException>(() => new InMemoryRateLimiter(clock, null!));
    }

    private static InMemoryRateLimiter NewLimiter(IClock clock, RateLimitingOptions options) =>
        new(clock, Microsoft.Extensions.Options.Options.Create(options));

    private static RateLimitingOptions NewOptions(int limit, int windowSeconds) => new()
    {
        Policies = new RateLimitPolicies(new[]
        {
            new RateLimitPolicyOptions { Name = "p", Limit = limit, WindowSeconds = windowSeconds },
        }),
    };
}
