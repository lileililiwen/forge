namespace Platform.Identity.OpenIdConnect.AspNetCore.Tests;

public class PlatformOpenIdConnectStateReplayGuardTests
{
    [Fact]
    public async Task A_state_is_accepted_once_and_rejected_afterwards()
    {
        var guard = new InMemoryPlatformOpenIdConnectStateReplayGuard();

        var first = await guard.TryConsumeAsync("state-value", CancellationToken.None);
        var second = await guard.TryConsumeAsync("state-value", CancellationToken.None);

        Assert.True(first);
        Assert.False(second);
    }

    [Fact]
    public async Task Different_states_are_accepted_independently()
    {
        var guard = new InMemoryPlatformOpenIdConnectStateReplayGuard();

        Assert.True(await guard.TryConsumeAsync("state-one", CancellationToken.None));
        Assert.True(await guard.TryConsumeAsync("state-two", CancellationToken.None));
    }

    [Fact]
    public async Task A_state_is_forgotten_after_its_lifetime()
    {
        var clock = new FakeTimeProvider(DateTimeOffset.Parse("2026-10-19T10:00:00Z"));
        var guard = new InMemoryPlatformOpenIdConnectStateReplayGuard(clock, TimeSpan.FromMinutes(10));

        Assert.True(await guard.TryConsumeAsync("state-value", CancellationToken.None));
        clock.Advance(TimeSpan.FromMinutes(11));

        Assert.True(await guard.TryConsumeAsync("state-value", CancellationToken.None));
    }

    [Fact]
    public void The_guard_refuses_a_lifetime_that_is_not_positive()
    {
        Assert.Throws<ArgumentOutOfRangeException>(
            () => new InMemoryPlatformOpenIdConnectStateReplayGuard(TimeProvider.System, TimeSpan.Zero));
    }
}

/// <summary>A clock the tests can move forward.</summary>
internal sealed class FakeTimeProvider : TimeProvider
{
    private DateTimeOffset _now;

    public FakeTimeProvider(DateTimeOffset now)
    {
        _now = now;
    }

    public override DateTimeOffset GetUtcNow() => _now;

    public void Advance(TimeSpan delta) => _now = _now.Add(delta);
}
